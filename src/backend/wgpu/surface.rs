//! Window surface, GPU device, swapchain configuration and presentation.

use std::sync::Arc;

use winit::window::Window as NativeWindow;

use crate::platform::{Window, WindowSize};
use crate::render::stats::RenderStats;
use crate::render::system::RenderError;

use super::checked_gpu_operation;
use super::pipeline::{MeshDraw, MeshPipeline, UniformBinding};
use super::texture::DepthTexture;

const DEVICE_LABEL: &str = "BFGE device";
const ENCODER_LABEL: &str = "BFGE frame encoder";
const RENDER_PASS_LABEL: &str = "BFGE mesh render pass";
const NO_CONFIGURATION: &str = "adapter does not support the window surface";
const ACQUISITION_FAILED: &str = "surface is unavailable after recovery";
const SURFACE_VALIDATION_FAILED: &str = "surface texture acquisition failed validation";
const MISSING_DEPTH_TEXTURE: &str = "drawable surface has no depth texture";
const MISSING_CAMERA_BINDING: &str = "mesh draws require a camera binding";
const TOO_MANY_DRAWS: &str = "frame draw count exceeds u32";
const TOO_MANY_TRIANGLES: &str = "frame triangle count exceeds u32";
const CLEAR_COLOR: wgpu::Color = wgpu::Color::BLACK;
const CLEAR_DEPTH: f32 = 1.0;
const ACQUISITION_ATTEMPTS: usize = 2;

/// A window-bound render surface with its device and presentation queue.
///
/// Zero-sized surfaces are suspended until resized to nonzero dimensions.
/// Draws frame geometry and presents it to the window.
#[derive(Debug)]
pub struct RenderSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: WindowSize,
    configured: bool,
    depth: Option<DepthTexture>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    instance: wgpu::Instance,
    window: Arc<NativeWindow>,
}

impl RenderSurface {
    pub(super) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(super) fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub(crate) fn new(window: &Window) -> Result<Self, RenderError> {
        let size = window.size();
        let native = window
            .native()
            .map_err(|error| RenderError::Initialization(error.to_string()))?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(Arc::clone(&native)),
        ));
        let surface = instance
            .create_surface(Arc::clone(&native))
            .map_err(|error| RenderError::Initialization(format!("create surface: {error}")))?;
        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .map_err(|error| {
                    RenderError::Initialization(format!("request adapter: {error}"))
                })?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some(DEVICE_LABEL),
                    ..Default::default()
                })
                .await
                .map_err(|error| RenderError::Initialization(format!("request device: {error}")))?;
            Ok::<_, RenderError>((adapter, device, queue))
        })?;
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| RenderError::Initialization(NO_CONFIGURATION.to_owned()))?;
        let mut result = Self {
            surface,
            config,
            size,
            configured: false,
            depth: None,
            adapter,
            device,
            queue,
            instance,
            window: native,
        };
        result
            .resize(size)
            .map_err(|error| RenderError::Initialization(format!("configure surface: {error}")))?;
        Ok(result)
    }

    pub(crate) fn resize(&mut self, size: WindowSize) -> Result<(), RenderError> {
        validate_surface_size(size, self.device.limits().max_texture_dimension_2d)?;
        if is_drawable(size) {
            let depth = DepthTexture::new(&self.device, size)?;
            let previous_dimensions = (self.config.width, self.config.height);
            self.config.width = size.width;
            self.config.height = size.height;
            match self.configure() {
                Ok(()) => {
                    self.size = size;
                    self.depth = Some(depth);
                    Ok(())
                }
                Err(error) => {
                    (self.config.width, self.config.height) = previous_dimensions;
                    Err(error)
                }
            }
        } else {
            self.size = size;
            self.configured = false;
            self.depth = None;
            Ok(())
        }
    }

    fn configure(&mut self) -> Result<(), RenderError> {
        self.configured = false;
        checked_gpu_operation(&self.device, || {
            self.surface.configure(&self.device, &self.config)
        })?;
        self.configured = true;
        Ok(())
    }

    pub(crate) fn render_frame(
        &mut self,
        pipeline: &MeshPipeline,
        camera: Option<&UniformBinding>,
        draws: &[MeshDraw<'_>],
    ) -> Result<RenderStats, RenderError> {
        let stats = draw_stats(draws)?;
        let camera = if draws.is_empty() {
            None
        } else {
            Some(camera.ok_or_else(|| RenderError::Render(MISSING_CAMERA_BINDING.to_owned()))?)
        };
        if is_drawable(self.size) {
            if !self.configured {
                self.configure()?;
            }
            if let Some(output) = self.acquire()? {
                let depth_view = self
                    .depth
                    .as_ref()
                    .ok_or_else(|| RenderError::Render(MISSING_DEPTH_TEXTURE.to_owned()))?
                    .view();
                checked_gpu_operation(&self.device, || {
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        self.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some(ENCODER_LABEL),
                            });
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(RENDER_PASS_LABEL),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: depth_view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(CLEAR_DEPTH),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            ..Default::default()
                        });
                        if let Some(camera) = camera {
                            encode_mesh_draws(&mut pass, pipeline, camera, draws);
                        }
                    }
                    self.queue.submit([encoder.finish()]);
                    self.window.pre_present_notify();
                    self.queue.present(output);
                })?;
                Ok(stats)
            } else {
                Ok(RenderStats::default())
            }
        } else {
            Ok(RenderStats::default())
        }
    }

    fn acquire(&mut self) -> Result<Option<wgpu::SurfaceTexture>, RenderError> {
        for _ in 0..ACQUISITION_ATTEMPTS {
            match checked_gpu_operation(&self.device, || self.surface.get_current_texture())? {
                wgpu::CurrentSurfaceTexture::Success(texture) => return Ok(Some(texture)),
                wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                    self.configured = false;
                    return Ok(Some(texture));
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(None);
                }
                wgpu::CurrentSurfaceTexture::Outdated => self.configure()?,
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.configured = false;
                    let surface = self
                        .instance
                        .create_surface(Arc::clone(&self.window))
                        .map_err(|error| {
                            RenderError::Render(format!("recreate surface: {error}"))
                        })?;
                    let config = surface
                        .get_default_config(&self.adapter, self.size.width, self.size.height)
                        .ok_or_else(|| RenderError::Render(NO_CONFIGURATION.to_owned()))?;
                    self.surface = surface;
                    self.config = config;
                    self.configure()?;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(RenderError::Render(SURFACE_VALIDATION_FAILED.to_owned()));
                }
            }
        }
        Err(RenderError::Render(ACQUISITION_FAILED.to_owned()))
    }
}

fn draw_stats(draws: &[MeshDraw<'_>]) -> Result<RenderStats, RenderError> {
    let mut stats = RenderStats::default();
    for draw in draws {
        for part in &draw.mesh.submeshes {
            stats.draw_calls = stats
                .draw_calls
                .checked_add(1)
                .ok_or_else(|| RenderError::Render(TOO_MANY_DRAWS.to_owned()))?;
            stats.triangles = stats
                .triangles
                .checked_add(part.index_count / 3)
                .ok_or_else(|| RenderError::Render(TOO_MANY_TRIANGLES.to_owned()))?;
        }
    }
    Ok(stats)
}

fn encode_mesh_draws(
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &MeshPipeline,
    camera: &UniformBinding,
    draws: &[MeshDraw<'_>],
) {
    pass.set_pipeline(&pipeline.render_pipeline);
    pass.set_bind_group(0, camera.group(), &[]);
    for draw in draws {
        pass.set_bind_group(1, draw.uniform.group(), &[]);
        for part in &draw.mesh.submeshes {
            pass.set_vertex_buffer(0, part.vertex_buffer.slice(..));
            pass.set_index_buffer(part.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..part.index_count, 0, 0..1);
        }
    }
}

fn is_drawable(size: WindowSize) -> bool {
    size.width != 0 && size.height != 0
}

fn validate_surface_size(size: WindowSize, limit: u32) -> Result<(), RenderError> {
    if size.width > limit || size.height > limit {
        Err(RenderError::Render(format!(
            "surface size {}x{} exceeds device limit {limit}",
            size.width, size.height
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::math::{Mat4, Vec3};
    use crate::render::frame::{RenderCamera, RenderItem};
    use crate::resources::manager::ResourceManager;
    use crate::resources::mesh::{Mesh, SubMesh, Vertex};

    use super::super::buffer::GpuMesh;
    use super::*;

    const TEST_WIDTH: u32 = 64;
    const TEST_HEIGHT: u32 = 64;
    const TEST_BYTES_PER_ROW: u32 = TEST_WIDTH * 4;

    #[test]
    fn minimized_surface_is_not_drawable() {
        assert!(!is_drawable(WindowSize::new(0, 600)));
        assert!(!is_drawable(WindowSize::new(800, 0)));
    }

    #[test]
    fn surface_size_at_device_limit_is_accepted() {
        assert!(validate_surface_size(WindowSize::new(2048, 2048), 2048).is_ok());
    }

    #[test]
    fn oversized_surface_is_rejected() {
        assert!(matches!(
            validate_surface_size(WindowSize::new(2049, 600), 2048),
            Err(RenderError::Render(_))
        ));
        assert!(matches!(
            validate_surface_size(WindowSize::new(800, 2049), 2048),
            Err(RenderError::Render(_))
        ));
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn shared_composite_mesh_draws_every_part_at_each_transform()
    -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
        let pipeline = MeshPipeline::new(&device, wgpu::TextureFormat::Rgba8Unorm)?;

        let part = |x: f32| SubMesh {
            vertices: [
                Vec3::new(x - 0.2, -0.2, 0.5),
                Vec3::new(x + 0.2, -0.2, 0.5),
                Vec3::new(x, 0.2, 0.5),
            ]
            .map(|position| Vertex { position })
            .to_vec(),
            indices: vec![0, 1, 2],
        };
        let mut resources = ResourceManager::new();
        let handle = resources.add_mesh(Mesh {
            submeshes: vec![part(-0.25), part(0.25)],
            color: [1.0, 0.0, 0.0, 1.0],
        });
        let mesh = GpuMesh::upload(&device, &resources, handle)?;
        let camera = pipeline.bind_camera(
            &device,
            &RenderCamera {
                view: Mat4::IDENTITY,
                projection: Mat4::IDENTITY,
            },
        )?;
        let draws = [-0.4, 0.4]
            .into_iter()
            .map(|x| {
                let item = RenderItem {
                    world_transform: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)),
                    mesh: handle,
                };
                Ok(MeshDraw {
                    mesh: &mesh,
                    uniform: pipeline.bind_mesh(&device, &item, mesh.color)?,
                })
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        assert_eq!(
            draw_stats(&draws)?,
            RenderStats {
                draw_calls: 4,
                triangles: 4
            }
        );

        let pixels = render_offscreen(&device, &queue, &pipeline, &camera, &draws)?;
        for x in [11, 27, 37, 53] {
            assert_eq!(pixel(&pixels, x, 32), &[255, 0, 0]);
        }
        assert_eq!(pixel(&pixels, 32, 32), &[0, 0, 0]);
        Ok(())
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn perspective_and_depth_keep_the_nearer_triangle_visible()
    -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
        let pipeline = MeshPipeline::new(&device, wgpu::TextureFormat::Rgba8Unorm)?;
        let triangle = SubMesh {
            vertices: [
                Vec3::new(-0.8, -0.8, 0.0),
                Vec3::new(0.8, -0.8, 0.0),
                Vec3::new(0.0, 0.8, 0.0),
            ]
            .map(|position| Vertex { position })
            .to_vec(),
            indices: vec![0, 1, 2],
        };
        let mut resources = ResourceManager::new();
        let near_handle = resources.add_mesh(Mesh {
            submeshes: vec![triangle.clone()],
            color: [1.0, 0.0, 0.0, 1.0],
        });
        let far_handle = resources.add_mesh(Mesh {
            submeshes: vec![triangle],
            color: [0.0, 1.0, 0.0, 1.0],
        });
        let near_mesh = GpuMesh::upload(&device, &resources, near_handle)?;
        let far_mesh = GpuMesh::upload(&device, &resources, far_handle)?;
        let camera = pipeline.bind_camera(
            &device,
            &RenderCamera {
                view: Mat4::IDENTITY,
                projection: crate::math::perspective(90.0_f32.to_radians(), 1.0),
            },
        )?;
        let near_item = RenderItem {
            world_transform: Mat4::from_translation(Vec3::new(0.0, 0.0, -1.0)),
            mesh: near_handle,
        };
        let far_item = RenderItem {
            world_transform: Mat4::from_translation(Vec3::new(0.8, 0.0, -2.0)),
            mesh: far_handle,
        };
        let draws = [
            MeshDraw {
                mesh: &near_mesh,
                uniform: pipeline.bind_mesh(&device, &near_item, near_mesh.color)?,
            },
            MeshDraw {
                mesh: &far_mesh,
                uniform: pipeline.bind_mesh(&device, &far_item, far_mesh.color)?,
            },
        ];

        let pixels = render_offscreen(&device, &queue, &pipeline, &camera, &draws)?;
        assert_eq!(pixel(&pixels, 22, 32), &[255, 0, 0]);
        assert_eq!(pixel(&pixels, 40, 32), &[255, 0, 0]);
        assert_eq!(pixel(&pixels, 49, 32), &[0, 255, 0]);
        assert_eq!(pixel(&pixels, 53, 32), &[0, 0, 0]);
        Ok(())
    }

    fn render_offscreen(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &MeshPipeline,
        camera: &UniformBinding,
        draws: &[MeshDraw<'_>],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let color_target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: TEST_WIDTH,
                height: TEST_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_target.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = DepthTexture::new(device, WindowSize::new(TEST_WIDTH, TEST_HEIGHT))?;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(TEST_BYTES_PER_ROW) * u64::from(TEST_HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let submission = checked_gpu_operation(device, || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth.view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                encode_mesh_draws(&mut pass, pipeline, camera, draws);
            }
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &color_target,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(TEST_BYTES_PER_ROW),
                        rows_per_image: Some(TEST_HEIGHT),
                    },
                },
                color_target.size(),
            );
            queue.submit([encoder.finish()])
        })?;
        let (sender, receiver) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(5)),
        })?;
        receiver.recv_timeout(Duration::from_secs(5))??;
        let pixels = readback.get_mapped_range(..)?;
        let result = pixels.to_vec();
        drop(pixels);
        readback.unmap();
        Ok(result)
    }

    fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
        let start = (y * TEST_WIDTH as usize + x) * 4;
        pixels
            .get(start..start + 3)
            .expect("sample coordinates fit the readback buffer")
    }
}

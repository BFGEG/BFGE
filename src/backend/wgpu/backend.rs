//! wgpu-backed graphics system.

use super::buffer::{MaterialCache, MeshCache, normalize_opacity};
use super::pipeline::MeshPipeline;
use super::surface::RenderSurface;
use super::texture::DepthTarget;
use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

const ALREADY_INITIALIZED: &str = "wgpu backend is already initialized";
const NOT_INITIALIZED: &str = "wgpu backend is not initialized";
const ENCODER_LABEL: &str = "BFGE frame encoder";
const PASS_LABEL: &str = "BFGE mesh pass";
const CLEAR_COLOR: wgpu::Color = wgpu::Color::BLACK;
const CLEAR_COLOR_LOAD: wgpu::Operations<wgpu::Color> = wgpu::Operations {
    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
    store: wgpu::StoreOp::Store,
};
const DEPTH_CLEAR: f32 = 1.0;
const DEPTH_LOAD: wgpu::Operations<f32> = wgpu::Operations {
    load: wgpu::LoadOp::Clear(DEPTH_CLEAR),
    store: wgpu::StoreOp::Store,
};

struct Draw {
    item: usize,
    part: usize,
    color: [f32; 4],
    transparent: bool,
    depth: f32,
}

fn sort_draws(draws: &mut [Draw]) {
    draws.sort_by(|a, b| {
        a.transparent.cmp(&b.transparent).then_with(|| {
            if a.transparent {
                b.depth.total_cmp(&a.depth)
            } else {
                std::cmp::Ordering::Equal
            }
        })
    });
}

/// Graphics system based on wgpu.
#[derive(Debug, Default)]
pub struct WgpuRenderSystem {
    surface: Option<RenderSurface>,
    pipeline: Option<MeshPipeline>,
    meshes: MeshCache,
    materials: MaterialCache,
    depth: DepthTarget,
    present_mode: Option<wgpu::PresentMode>,
}

impl WgpuRenderSystem {
    /// Creates a wgpu render system.
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "stress-benchmark")]
    pub(crate) fn with_present_mode(present_mode: wgpu::PresentMode) -> Self {
        Self {
            present_mode: Some(present_mode),
            ..Self::default()
        }
    }

    #[cfg(feature = "stress-benchmark")]
    pub(crate) fn adapter_info(&self) -> Option<wgpu::AdapterInfo> {
        self.surface.as_ref().map(RenderSurface::adapter_info)
    }
}

impl RenderSystem for WgpuRenderSystem {
    fn initialize(&mut self, window: &Window) -> Result<(), RenderError> {
        if self.surface.is_some() {
            Err(RenderError::Initialization(ALREADY_INITIALIZED.to_owned()))
        } else {
            self.surface = Some(RenderSurface::new_with_present_mode(
                window,
                self.present_mode,
            )?);
            self.pipeline = None;
            self.meshes = MeshCache::default();
            self.materials = MaterialCache::default();
            self.depth = DepthTarget::default();
            Ok(())
        }
    }

    fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError> {
        self.surface
            .as_mut()
            .ok_or_else(|| RenderError::Render(NOT_INITIALIZED.to_owned()))?
            .resize(size)
    }

    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        let Self {
            surface,
            pipeline,
            meshes,
            materials,
            depth,
            present_mode: _,
        } = self;
        let surface = surface
            .as_mut()
            .ok_or_else(|| RenderError::Render(NOT_INITIALIZED.to_owned()))?;

        let Some(output) = surface.begin_frame()? else {
            return Ok(RenderStats::default());
        };

        let device = surface.device();
        let queue = surface.queue();
        let pipeline = match pipeline {
            Some(pipeline) => pipeline,
            None => pipeline.insert(MeshPipeline::new(device, surface.format())),
        };

        let size = output.size;
        let Some(depth_view) = depth.view(device, size.width, size.height)?.cloned() else {
            surface.present(output);
            return Ok(RenderStats::default());
        };

        pipeline.write_camera(
            queue,
            frame.camera.projection * frame.camera.view,
            frame.lighting,
        );
        let mut draws = Vec::new();
        for (index, item) in frame.items.iter().enumerate() {
            let mesh = meshes.get_mesh(device, resources, item.mesh)?;
            for (part, submesh) in mesh.submeshes.iter().enumerate() {
                let material = materials.get_material(
                    device,
                    queue,
                    resources,
                    pipeline.material_layout(),
                    item.mesh,
                    submesh.material,
                )?;
                let alpha = material
                    .opacity
                    .unwrap_or_else(|| normalize_opacity(mesh.color[3]));
                if alpha == 0.0 {
                    continue;
                }
                let [r, g, b] =
                    item.color_override
                        .unwrap_or([mesh.color[0], mesh.color[1], mesh.color[2]]);
                let center =
                    (frame.camera.view * item.world_transform).transform_point3(submesh.center);
                draws.push(Draw {
                    item: index,
                    part,
                    color: [r, g, b, alpha],
                    transparent: alpha < 1.0 || material.texture_transparent,
                    depth: -center.z,
                });
            }
        }
        sort_draws(&mut draws);
        pipeline.reserve(device, draws.len() as u32);
        for (index, draw) in draws.iter().enumerate() {
            pipeline.write_mesh(
                queue,
                index as u32,
                frame.items[draw.item].world_transform,
                draw.color,
            );
        }
        let mut stats = RenderStats::default();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(ENCODER_LABEL),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: CLEAR_COLOR_LOAD,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(DEPTH_LOAD),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, pipeline.camera_bind_group(), &[]);
            for (index, draw) in draws.iter().enumerate() {
                let item = &frame.items[draw.item];
                let mesh = meshes.get_mesh(device, resources, item.mesh)?;
                let submesh = &mesh.submeshes[draw.part];
                let material = materials.get_material(
                    device,
                    queue,
                    resources,
                    pipeline.material_layout(),
                    item.mesh,
                    submesh.material,
                )?;
                pass.set_pipeline(pipeline.pipeline(draw.transparent));
                pass.set_bind_group(
                    1,
                    pipeline.mesh_bind_group(),
                    &[index as u32 * pipeline.mesh_stride()],
                );
                pass.set_bind_group(2, &material.bind_group, &[]);
                pass.set_vertex_buffer(0, submesh.vertex_buffer.slice(..));
                pass.set_index_buffer(submesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..submesh.index_count, 0, 0..1);
                stats.draw_calls += 1;
                stats.triangles += submesh.index_count / 3;
            }
        }

        queue.submit([encoder.finish()]);
        surface.present(output);
        Ok(stats)
    }

    fn shutdown(&mut self) {
        self.surface = None;
        self.pipeline = None;
        self.depth = DepthTarget::default();
    }
}

#[cfg(test)]
mod tests {
    use crate::math::Mat4;
    use crate::render::frame::RenderCamera;
    use crate::scene::lighting::SceneLighting;

    use super::*;

    #[test]
    fn transparent_parts_follow_opaque_and_sort_by_view_depth() {
        let make = |item, transparent, depth| Draw {
            item,
            part: 0,
            color: [1.0; 4],
            transparent,
            depth,
        };
        let mut draws = [
            make(0, true, 2.0),
            make(1, false, 1.0),
            make(2, true, 8.0),
            make(3, false, 9.0),
            make(4, true, 2.0),
        ];
        sort_draws(&mut draws);
        assert_eq!(draws.map(|draw| draw.item), [1, 3, 2, 0, 4]);
    }

    #[test]
    fn resize_requires_initialization() {
        let mut backend = WgpuRenderSystem::new();
        assert!(matches!(backend.set_window_size(WindowSize::new(800, 600)),
            Err(RenderError::Render(message)) if message == NOT_INITIALIZED
        ));
    }

    #[test]
    fn render_requires_initialization() {
        let mut backend = WgpuRenderSystem::new();
        let frame = RenderFrame {
            camera: RenderCamera {
                view: Mat4::IDENTITY,
                projection: Mat4::IDENTITY,
            },
            lighting: SceneLighting::default(),
            items: Vec::new(),
        };
        assert!(matches!(backend.render(&frame, &ResourceManager::new()),
            Err(RenderError::Render(message)) if message == NOT_INITIALIZED
        ));
    }

    #[test]
    fn shutdown_without_initialization_is_idempotent() {
        let mut backend = WgpuRenderSystem::new();
        backend.shutdown();
        backend.shutdown();
        assert!(matches!(backend.set_window_size(WindowSize::new(800, 600)),
            Err(RenderError::Render(message)) if message == NOT_INITIALIZED
        ));
    }
}

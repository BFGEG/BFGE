//! wgpu-backed graphics system.

use super::buffer::MeshBufferCache;
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

/// Graphics system based on wgpu.
#[derive(Debug, Default)]
pub struct WgpuRenderSystem {
    surface: Option<RenderSurface>,
    pipeline: Option<MeshPipeline>,
    buffers: MeshBufferCache,
    depth: DepthTarget,
}

impl WgpuRenderSystem {
    /// Creates a wgpu render system.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RenderSystem for WgpuRenderSystem {
    fn initialize(&mut self, window: &Window) -> Result<(), RenderError> {
        if self.surface.is_some() {
            Err(RenderError::Initialization(ALREADY_INITIALIZED.to_owned()))
        } else {
            self.surface = Some(RenderSurface::new(window)?);
            self.pipeline = None;
            self.buffers = MeshBufferCache::default();
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
            buffers,
            depth,
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

        pipeline.reserve(device, frame.items.len() as u32);
        pipeline.write_camera(
            queue,
            frame.camera.projection * frame.camera.view,
            frame.lighting,
        );
        let mut stats = RenderStats::default();
        for (index, item) in frame.items.iter().enumerate() {
            let mesh = buffers.get(device, resources, item.mesh)?;
            pipeline.write_mesh(queue, index as u32, item.world_transform, mesh.color);
            stats.triangles += mesh
                .submeshes
                .iter()
                .map(|submesh| submesh.index_count / 3)
                .sum::<u32>();
        }

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
            if !frame.items.is_empty() {
                pass.set_pipeline(pipeline.pipeline());
                pass.set_bind_group(0, pipeline.camera_bind_group(), &[]);
                for (index, item) in frame.items.iter().enumerate() {
                    let mesh = buffers.get(device, resources, item.mesh)?;
                    let offset = index as u32 * pipeline.mesh_stride();
                    pass.set_bind_group(1, pipeline.mesh_bind_group(), &[offset]);
                    for submesh in &mesh.submeshes {
                        pass.set_vertex_buffer(0, submesh.vertices.slice(..));
                        pass.set_index_buffer(submesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..submesh.index_count, 0, 0..1);
                        stats.draw_calls += 1;
                    }
                }
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

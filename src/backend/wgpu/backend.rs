//! wgpu-backed graphics system.

use std::collections::{HashMap, hash_map::Entry};

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

use super::buffer::GpuMesh;
use super::pipeline::{MeshDraw, MeshPipeline};
use super::surface::RenderSurface;

const ALREADY_INITIALIZED: &str = "wgpu backend is already initialized";
const NOT_INITIALIZED: &str = "wgpu backend is not initialized";

/// Graphics system based on wgpu.
///
/// Initializes the GPU and draws frame meshes into the window surface.
#[derive(Debug, Default)]
pub struct WgpuRenderSystem {
    state: Option<WgpuState>,
}

#[derive(Debug)]
struct WgpuState {
    surface: RenderSurface,
    pipeline: MeshPipeline,
}

impl WgpuRenderSystem {
    /// Creates a wgpu render system.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RenderSystem for WgpuRenderSystem {
    fn initialize(&mut self, window: &Window) -> Result<(), RenderError> {
        if self.state.is_some() {
            Err(RenderError::Initialization(ALREADY_INITIALIZED.to_owned()))
        } else {
            let surface = RenderSurface::new(window)?;
            let pipeline = MeshPipeline::new(surface.device(), surface.format())?;
            self.state = Some(WgpuState { surface, pipeline });
            Ok(())
        }
    }

    fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError> {
        self.state
            .as_mut()
            .ok_or_else(|| RenderError::Render(NOT_INITIALIZED.to_owned()))?
            .surface
            .resize(size)
    }

    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| RenderError::Render(NOT_INITIALIZED.to_owned()))?;

        let mut meshes = HashMap::new();
        for item in &frame.items {
            if let Entry::Vacant(entry) = meshes.entry(item.mesh) {
                entry.insert(GpuMesh::upload(
                    state.surface.device(),
                    resources,
                    item.mesh,
                )?);
            }
        }
        let camera = frame
            .items
            .first()
            .map(|_| {
                state
                    .pipeline
                    .bind_camera(state.surface.device(), &frame.camera)
            })
            .transpose()?;
        let mut draws = Vec::with_capacity(frame.items.len());
        for item in &frame.items {
            let mesh = meshes.get(&item.mesh).ok_or_else(|| {
                RenderError::Render(format!("mesh {:?} was not uploaded", item.mesh))
            })?;
            let uniform = state
                .pipeline
                .bind_mesh(state.surface.device(), item, mesh.color)?;
            draws.push(MeshDraw { mesh, uniform });
        }

        state
            .surface
            .render_frame(&state.pipeline, camera.as_ref(), &draws)
    }

    fn shutdown(&mut self) {
        self.state = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::math::Mat4;
    use crate::render::frame::RenderCamera;

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

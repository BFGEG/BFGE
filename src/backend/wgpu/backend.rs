//! wgpu-backed graphics system.

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

use super::surface::RenderSurface;

const ALREADY_INITIALIZED: &str = "wgpu backend is already initialized";
const NOT_INITIALIZED: &str = "wgpu backend is not initialized";
const DRAWING_UNAVAILABLE: &str = "mesh drawing is not implemented (issue #13)";

/// Graphics system based on wgpu.
///
/// Initializes the GPU and presents cleared frames. Mesh drawing is deferred
/// to issue #13; nonempty frames return an error instead of dropping geometry.
#[derive(Debug, Default)]
pub struct WgpuRenderSystem {
    surface: Option<RenderSurface>,
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
        _resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        let surface = self
            .surface
            .as_mut()
            .ok_or_else(|| RenderError::Render(NOT_INITIALIZED.to_owned()))?;
        if frame.items.is_empty() {
            surface.clear_and_present()?;
            Ok(RenderStats::default())
        } else {
            Err(RenderError::Render(DRAWING_UNAVAILABLE.to_owned()))
        }
    }

    fn shutdown(&mut self) {
        self.surface = None;
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

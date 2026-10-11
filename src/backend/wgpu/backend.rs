//! wgpu-backed graphics system.

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

/// Graphics system based on wgpu.
///
/// Actual wgpu initialization and rendering are the next milestone.
#[derive(Debug, Default)]
pub struct WgpuRenderSystem;

impl WgpuRenderSystem {
    /// Creates a wgpu render system.
    pub fn new() -> Self {
        Self
    }
}

impl RenderSystem for WgpuRenderSystem {
    fn initialize(&mut self, _window: &Window) -> Result<(), RenderError> {
        todo!("wgpu: milestone 2")
    }

    fn set_window_size(&mut self, _size: WindowSize) -> Result<(), RenderError> {
        todo!("wgpu: milestone 2")
    }

    fn render(
        &mut self,
        _frame: &RenderFrame,
        _resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        todo!("wgpu: milestone 2")
    }

    fn shutdown(&mut self) {}
}

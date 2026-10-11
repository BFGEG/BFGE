//! wgpu-backed graphics system.

use crate::render::frame::RenderFrame;
use crate::render::image::ImageSize;
use crate::render::system::{RenderError, RenderOutput, RenderSystem};
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
    fn initialize(&mut self) -> Result<(), RenderError> {
        todo!("wgpu: milestone 2")
    }

    fn resize(&mut self, _size: ImageSize) -> Result<(), RenderError> {
        todo!("wgpu: milestone 2")
    }

    fn render(
        &mut self,
        _frame: &RenderFrame,
        _resources: &ResourceManager,
    ) -> Result<RenderOutput, RenderError> {
        todo!("wgpu: milestone 2")
    }

    fn shutdown(&mut self) {}
}

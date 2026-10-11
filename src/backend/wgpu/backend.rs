//! wgpu-backed graphics system.

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

const INITIALIZATION_UNAVAILABLE: &str = "wgpu initialization is not implemented (issue #10)";
const RESIZE_UNAVAILABLE: &str = "wgpu surface resizing is not implemented (issue #10)";
const RENDERING_UNAVAILABLE: &str =
    "wgpu rendering and presentation are not implemented (issue #10)";

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
        Err(RenderError::Initialization(
            INITIALIZATION_UNAVAILABLE.to_owned(),
        ))
    }

    fn set_window_size(&mut self, _size: WindowSize) -> Result<(), RenderError> {
        Err(RenderError::Render(RESIZE_UNAVAILABLE.to_owned()))
    }

    fn render(
        &mut self,
        _frame: &RenderFrame,
        _resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        Err(RenderError::Render(RENDERING_UNAVAILABLE.to_owned()))
    }

    fn shutdown(&mut self) {}
}

//! Owner of the graphics system.

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

/// Owner of the graphics system.
pub struct Renderer {
    render_system: Box<dyn RenderSystem>,
}

impl Renderer {
    /// Creates a renderer over the given graphics system.
    pub fn new(render_system: Box<dyn RenderSystem>) -> Self {
        Self { render_system }
    }

    /// # Errors - see [`RenderSystem::initialize`].
    pub fn initialize(&mut self, window: &Window) -> Result<(), RenderError> {
        self.render_system.initialize(window)
    }

    /// # Errors - see [`RenderSystem::set_window_size`].
    pub fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError> {
        self.render_system.set_window_size(size)
    }

    /// # Errors - see [`RenderSystem::render`].
    pub fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        self.render_system.render(frame, resources)
    }

    /// Shuts the graphics system down.
    pub fn shutdown(&mut self) {
        self.render_system.shutdown();
    }
}

//! Owner of the graphics system.

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::manager::ResourceManager;

/// Coordinates frame submission and window lifecycle for a graphics system.
pub struct Renderer {
    render_system: Box<dyn RenderSystem>,
}

impl Renderer {
    /// Creates a renderer that delegates to `render_system`.
    pub fn new(render_system: Box<dyn RenderSystem>) -> Self {
        Self { render_system }
    }

    /// Initializes the graphics system for `window`.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend cannot initialize its graphics resources.
    pub fn initialize(&mut self, window: &Window) -> Result<(), RenderError> {
        self.render_system.initialize(window)
    }

    /// Applies a new window size to the graphics system.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend cannot apply the size.
    pub fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError> {
        self.render_system.set_window_size(size)
    }

    /// Renders `frame` using geometry registered in `resources`.
    ///
    /// Returns statistics for the render pass. A backend may return zeroed
    /// statistics when a frame cannot be presented temporarily.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend cannot render the frame or resolve a
    /// referenced mesh.
    pub fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        self.render_system.render(frame, resources)
    }

    /// Releases graphics resources held by the graphics system.
    pub fn shutdown(&mut self) {
        self.render_system.shutdown();
    }
}

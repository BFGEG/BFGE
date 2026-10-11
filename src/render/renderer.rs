//! Owner of the graphics system.

use crate::render::frame::RenderFrame;
use crate::render::image::ImageSize;
use crate::render::system::{RenderError, RenderOutput, RenderSystem};
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
    pub fn initialize(&mut self) -> Result<(), RenderError> {
        self.render_system.initialize()
    }

    /// # Errors - see [`RenderSystem::resize`].
    pub fn resize(&mut self, size: ImageSize) -> Result<(), RenderError> {
        self.render_system.resize(size)
    }

    /// # Errors - see [`RenderSystem::render`].
    pub fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderOutput, RenderError> {
        self.render_system.render(frame, resources)
    }

    /// Shuts the graphics system down.
    pub fn shutdown(&mut self) {
        self.render_system.shutdown();
    }
}

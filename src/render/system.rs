//! Graphics system interface (the backend replacement point).

use crate::platform::{Window, WindowSize};
use crate::render::frame::RenderFrame;
use crate::render::stats::RenderStats;
use crate::resources::manager::ResourceManager;

/// Graphics system error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// Failed to initialize the graphics system.
    #[error("failed to initialize the graphics system: {0}")]
    Initialization(String),
    /// Failed to render a frame.
    #[error("failed to render a frame: {0}")]
    Render(String),
}

/// Graphics system interface (the backend replacement point).
pub trait RenderSystem {
    /// # Errors - [`RenderError::Initialization`].
    fn initialize(&mut self, window: &Window) -> Result<(), RenderError>;
    /// # Errors - [`RenderError`].
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError>;
    /// # Errors - [`RenderError`].
    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError>;
    /// Shuts the graphics system down.
    fn shutdown(&mut self);
}

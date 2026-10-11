//! Graphics system interface (the backend replacement point).

use crate::render::frame::RenderFrame;
use crate::render::image::{ImageSize, SceneImage};
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
    /// Failed to capture the frame image.
    #[error("failed to capture the frame image: {0}")]
    ImageCapture(String),
}

/// Frame rendering result.
#[derive(Debug)]
pub struct RenderOutput {
    /// Frame statistics.
    pub stats: RenderStats,
    /// Ready frame image.
    pub image: SceneImage,
}

/// Graphics system interface (the backend replacement point).
pub trait RenderSystem {
    /// # Errors - [`RenderError::Initialization`].
    fn initialize(&mut self) -> Result<(), RenderError>;
    /// # Errors - [`RenderError`].
    fn resize(&mut self, size: ImageSize) -> Result<(), RenderError>;
    /// # Errors - [`RenderError`].
    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderOutput, RenderError>;
    /// Shuts the graphics system down.
    fn shutdown(&mut self);
}

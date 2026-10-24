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

/// Interface implemented by a graphics backend.
///
/// The renderer calls these methods over a window session: initialize once,
/// apply size changes as they arrive, render frames, then shut down.
/// Implementations own their backend-specific graphics resources.
pub trait RenderSystem {
    /// Initializes backend resources for the engine window.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Initialization`] if the backend cannot be set up.
    fn initialize(&mut self, window: &Window) -> Result<(), RenderError>;
    /// Applies the current window size to the backend surface.
    ///
    /// A zero width or height suspends rendering until a drawable size is
    /// received.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if the backend cannot apply the size.
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError>;
    /// Renders and presents a frame using geometry from `resources`.
    ///
    /// Return zeroed statistics if the surface temporarily cannot present a
    /// frame.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if rendering fails or a referenced mesh cannot
    /// be resolved.
    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError>;
    /// Releases backend resources and returns the implementation to its
    /// uninitialized state.
    fn shutdown(&mut self);
}

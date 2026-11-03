//! Engine error type.

use crate::platform::PlatformError;
use crate::render::system::RenderError;
use crate::scene::error::SceneError;
use crate::systems::lifecycle::LifecycleError;
use crate::systems::loading::ModelLoadError;

/// Error of the engine facade.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The engine is already initialized.
    #[error("engine is already initialized")]
    AlreadyInitialized,
    /// The engine is not initialized.
    #[error("engine is not initialized")]
    NotInitialized,
    /// Scene error (e.g. a frame handler error).
    #[error("scene error: {0}")]
    Scene(#[from] SceneError),
    /// Model loading failed in a frame handler.
    #[error("model loading error: {0}")]
    ModelLoad(#[from] ModelLoadError),
    /// Graphics system error.
    #[error("graphics system error: {0}")]
    Render(#[from] RenderError),
    /// Lifecycle error.
    #[error("lifecycle error: {0}")]
    Lifecycle(#[from] LifecycleError),
    /// Platform (window) error.
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),
}

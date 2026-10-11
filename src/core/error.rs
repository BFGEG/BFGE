//! Engine error type.

use crate::render::system::RenderError;
use crate::systems::lifecycle::LifecycleError;

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
    /// Graphics system error.
    #[error("graphics system error: {0}")]
    Render(#[from] RenderError),
    /// Lifecycle error.
    #[error("lifecycle error: {0}")]
    Lifecycle(#[from] LifecycleError),
}

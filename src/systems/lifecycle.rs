//! Assembly, initialization and shutdown of engine parts.

use crate::platform::Window;
use crate::render::renderer::Renderer;
use crate::render::system::RenderError;
use crate::resources::manager::ResourceManager;
use crate::scene::manager::SceneManager;

use super::persistence::SceneStore;

/// Assembled engine parts for the engine's runtime.
pub struct EngineParts {
    /// Scene manager.
    pub scene: SceneManager,
    /// Resource manager.
    pub resources: ResourceManager,
    /// Renderer.
    pub renderer: Renderer,
    /// Scene store (save/restore).
    pub store: SceneStore,
}

/// Assembles the engine parts (without initializing the graphics).
pub fn create() -> EngineParts {
    // SceneManager::new(), ResourceManager::new(),
    // Renderer::new(Box::new(WgpuRenderSystem::new())), SceneStore::new()
    todo!("milestone 2")
}

/// Initializes the graphics system and creates a render surface for the
/// engine's window.
///
/// # Errors
///
/// Returns [`LifecycleError`] on initialization failure.
pub fn initialize(_parts: &mut EngineParts, _window: &Window) -> Result<(), LifecycleError> {
    todo!("milestone 2")
}

/// Shuts the engine parts down.
///
/// # Errors
///
/// Returns [`LifecycleError`] on shutdown failure.
pub fn shutdown(_parts: &mut EngineParts) -> Result<(), LifecycleError> {
    todo!("milestone 2")
}

/// Error of the engine parts lifecycle.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LifecycleError {
    /// Graphics system error.
    #[error("graphics system error: {0}")]
    Render(#[from] RenderError),
}

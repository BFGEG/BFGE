//! Assembly, initialization and shutdown of engine parts.

use crate::backend::wgpu::backend::WgpuRenderSystem;
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
    EngineParts {
        scene: SceneManager::new(),
        resources: ResourceManager::new(),
        renderer: Renderer::new(Box::new(WgpuRenderSystem::new())),
        store: SceneStore::new(),
    }
}

/// Initializes the graphics system and creates a render surface for the
/// engine's window.
///
/// # Errors
///
/// Returns [`LifecycleError`] on initialization failure.
pub fn initialize(parts: &mut EngineParts, window: &Window) -> Result<(), LifecycleError> {
    parts.renderer.initialize(window)?;
    Ok(())
}

/// Shuts the engine parts down and resets the scene and resources.
///
/// The renderer is shut down first, then the scene and resources are reset to
/// their initial state. The scene store is preserved so a previous save can be
/// restored after reinitialization. This operation currently has no failure
/// path.
pub fn shutdown(parts: &mut EngineParts) -> Result<(), LifecycleError> {
    parts.renderer.shutdown();
    parts.scene = SceneManager::new();
    parts.resources = ResourceManager::new();
    Ok(())
}

/// Error of the engine parts lifecycle.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LifecycleError {
    /// Graphics system error.
    #[error("graphics system error: {0}")]
    Render(#[from] RenderError),
}

//! Scene save and restore.

use crate::resources::manager::ResourceManager;
use crate::scene::manager::SceneManager;

/// Storage for a single saved scene (version 1 - in engine memory).
#[derive(Debug, Default)]
pub struct SceneStore;

impl SceneStore {
    /// Creates an empty scene store.
    pub fn new() -> Self {
        Self
    }
}

/// Scene save or restore error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ScenePersistenceError {
    /// The scene is not empty: restore is only possible into an empty scene.
    #[error("scene is not empty: restore is only possible into an empty scene")]
    SceneNotEmpty,
    /// Failed to save the scene.
    #[error("failed to save the scene: {0}")]
    Save(String),
    /// Failed to restore the scene.
    #[error("failed to restore the scene: {0}")]
    Restore(String),
}

/// Saves the full scene state (replaces the previous save).
///
/// # Errors
///
/// Returns [`ScenePersistenceError`] if the full save fails.
pub fn save_scene(
    _store: &mut SceneStore,
    _scene: &SceneManager,
    _resources: &ResourceManager,
) -> Result<(), ScenePersistenceError> {
    todo!("milestone 2")
}

/// Restores the saved scene into the empty current scene.
///
/// # Errors
///
/// Returns [`ScenePersistenceError`] on failure.
pub fn restore_scene(
    _store: &mut SceneStore,
    _scene: &mut SceneManager,
    _resources: &mut ResourceManager,
) -> Result<(), ScenePersistenceError> {
    todo!("milestone 2")
}

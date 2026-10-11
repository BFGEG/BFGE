//! Client-facing engine operations and the image listener.

use std::path::Path;

use crate::core::error::EngineError;
use crate::render::image::{ImageSize, SceneImage};
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::systems::loading::ModelLoadError;
use crate::systems::persistence::ScenePersistenceError;

/// Public operations to drive the engine and exchange data with the client.
pub trait EngineApi {
    /// Initializes the engine internals and creates the initial scene.
    ///
    /// The initial scene contains a root node and a viewpoint with default
    /// values, but no scene objects.
    fn initialize(&mut self, initial_image_size: ImageSize) -> Result<(), EngineError>;

    /// Shuts the engine and its internals down.
    fn shutdown(&mut self) -> Result<(), EngineError>;

    /// Returns access to operations over the current scene.
    fn scene(&mut self) -> &mut dyn SceneApi;

    /// Loads a model from a file and creates a separate scene object for it.
    ///
    /// The object is immediately linked to the given node. Every call creates
    /// a new object, including repeated loads of the same file.
    fn load_model(
        &mut self,
        path: &Path,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, ModelLoadError>;

    /// Sets the size of the image produced by the engine.
    ///
    /// May be called while the engine is running.
    fn set_image_size(&mut self, size: ImageSize) -> Result<(), EngineError>;

    /// Assigns the receiver of images that become ready in the engine loop.
    ///
    /// May be called before initialization to receive the first image.
    fn set_image_listener(&mut self, listener: Box<dyn ImageListener>);

    /// Saves the full scene state to the engine-defined storage.
    fn save_scene(&mut self) -> Result<(), ScenePersistenceError>;

    /// Restores the saved scene if the current scene contains no objects.
    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError>;
}

/// Receiver of notifications about ready scene images.
pub trait ImageListener {
    /// Called by the engine when a new scene image is ready.
    fn image_ready(&mut self, image: &SceneImage);
}

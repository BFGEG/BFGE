//! Client-facing engine operations.

use std::path::Path;

use crate::core::engine_loop::EngineEvent;
use crate::core::error::EngineError;
use crate::platform::{WindowDescriptor, WindowSize};
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::systems::loading::ModelLoadError;
use crate::systems::persistence::ScenePersistenceError;

/// Public operations to drive the engine and exchange data with the client.
pub trait EngineApi {
    /// Initializes the engine internals, creates the window and the initial
    /// scene.
    ///
    /// Call this on the application's main thread. Explicit window dimensions
    /// must be nonzero; platform and graphics failures are returned as errors.
    ///
    /// The window is created by the engine; the client does not manage the
    /// windowing or graphics platform. The initial scene contains a root node
    /// and a viewpoint with default values, but no scene objects.
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError>;

    /// Runs the engine's main loop until the window is closed.
    ///
    /// The loop updates the scene and presents it in the window; the client
    /// does not drive individual frame steps. Returns after the window is
    /// closed, on a frame handler error, or on an unrecoverable error.
    fn run(&mut self) -> Result<(), EngineError>;

    /// Registers user frame handlers.
    ///
    /// `frame_started` is called before the update and frame build;
    /// `frame_ended` is called after the frame is presented. Handlers are
    /// called in registration order and receive access to the scene and frame
    /// times. A handler error stops the loop and is returned from `run`.
    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError>;

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

    /// Sets the size of the engine's window.
    ///
    /// May be called while the engine is running; the window and the render
    /// surface are rebuilt for the new size.
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError>;

    /// Saves the full scene state to the engine-defined storage.
    fn save_scene(&mut self) -> Result<(), ScenePersistenceError>;

    /// Restores the saved scene if the current scene contains no objects.
    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError>;
}

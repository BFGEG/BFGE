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
    /// and a viewpoint and lighting with default values, but no scene objects.
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError>;

    /// Runs the engine's main loop until the window is closed.
    ///
    /// The loop updates the scene and presents it in the window; the client
    /// does not drive individual frame steps. Returns after the window is
    /// closed, on a frame handler error, or on an unrecoverable error.
    /// The engine remains initialized until [`Self::shutdown`] is called.
    /// A failed run clears pending events; registered handlers are retained.
    fn run(&mut self) -> Result<(), EngineError>;

    /// Registers user frame handlers.
    ///
    /// `frame_started` is called before frame build;
    /// `frame_ended` is called after a successful render attempt. Handlers are
    /// called in registration order and receive access to the scene and frame
    /// times. A handler error stops the loop and is returned from `run`.
    /// Applied scene changes are retained even if a later handler fails.
    /// Both handler phases are paused while the window has a zero dimension.
    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError>;

    /// Shuts the engine and its internals down.
    fn shutdown(&mut self) -> Result<(), EngineError>;

    /// Returns access to operations over the current scene.
    fn scene(&mut self) -> &mut dyn SceneApi;

    /// Loads a model from a file and creates a separate scene object for it.
    ///
    /// The object is immediately linked to the given node. Every call creates
    /// a new object, including repeated loads of the same file.
    /// Supports `.gltf`, `.glb` and `.obj`; parts share the first explicit color
    /// on used geometry, or neutral linear RGBA `[0.8, 0.8, 0.8, 1.0]`.
    /// Textures and model-provided normals are ignored. Scene lighting is
    /// applied when the model is rendered. This operation requires no window.
    ///
    /// # Errors
    ///
    /// Returns [`ModelLoadError`] for an unknown parent, unsupported format or
    /// geometry, unreadable model/buffer/material file, or corrupted data.
    /// A failed load leaves the scene and resources unchanged.
    fn load_model(
        &mut self,
        path: &Path,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, ModelLoadError>;

    /// Sets the size of the engine's window.
    ///
    /// Call after `initialize` and before `shutdown`. The blocking `run` method
    /// holds mutable access to the engine, so the client cannot call this
    /// operation during that run. Native window resize events are handled by
    /// the engine loop.
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError>;

    /// Saves the full scene state to the engine-defined storage.
    fn save_scene(&mut self) -> Result<(), ScenePersistenceError>;

    /// Restores the saved scene if the current scene contains no objects.
    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError>;
}

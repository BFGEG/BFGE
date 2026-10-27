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
    /// Initializes the graphics system and creates the engine's window.
    ///
    /// Call this on the application's main thread. Explicit window dimensions
    /// must be nonzero; platform and graphics failures are returned as errors.
    ///
    /// The window is created by the engine; the client does not manage the
    /// windowing or graphics platform. The scene created with the engine is
    /// retained during initialization. It contains a root node, a default
    /// viewpoint and default lighting, but no scene objects.
    ///
    /// # Errors
    ///
    /// Returns an error if a session is already active, the window dimensions
    /// are invalid, or window or graphics initialization fails.
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError>;

    /// Runs the engine's main loop until the window is closed or an error occurs.
    ///
    /// The loop updates the scene and presents it in the window; the client
    /// does not drive individual frame steps. Returns after the window is
    /// closed, on a frame handler error, or on an unrecoverable error.
    /// The engine remains initialized until [`Self::shutdown`] is called.
    /// Pending internal events are cleared when the loop exits; registered
    /// handlers are retained.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine is not initialized, a frame handler fails,
    /// or platform or graphics processing fails.
    fn run(&mut self) -> Result<(), EngineError>;

    /// Registers user frame handlers for this engine instance.
    ///
    /// `frame_started` is called before frame build;
    /// `frame_ended` is called after a successful render attempt. Handlers are
    /// called in registration order and receive access to the scene and frame
    /// times. A handler error stops the loop and is returned from `run`. Scene
    /// changes made before an error are retained. Handlers remain registered
    /// across sessions. Both phases are paused while the window has a zero
    /// dimension.
    ///
    /// # Returns
    ///
    /// Registration currently succeeds for every event.
    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError>;

    /// Ends the active session and resets its scene and model resources.
    ///
    /// The saved scene and registered frame handlers remain available on the
    /// engine instance. Calling this when no session is active returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInitialized`] when there is no active session.
    fn shutdown(&mut self) -> Result<(), EngineError>;

    /// Returns access to operations over the current scene.
    ///
    /// The scene is available before window initialization and after shutdown.
    ///
    /// # Returns
    ///
    /// Mutable access to the current scene operations.
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
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInitialized`] if no window session is active,
    /// or a platform or graphics error if applying the size fails.
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError>;

    /// Replaces the in-memory snapshot with the current scene state.
    ///
    /// The snapshot includes nodes, objects, loaded geometry, viewpoint and
    /// lighting. It survives session shutdown but not destruction of the engine.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after replacing the snapshot.
    fn save_scene(&mut self) -> Result<(), ScenePersistenceError>;

    /// Restores the saved state when the current scene contains no objects.
    ///
    /// A saved snapshot replaces the current nodes, viewpoint, lighting and
    /// resources. If no snapshot exists, the scene remains unchanged. Existing
    /// nodes without objects do not prevent restoration.
    ///
    /// # Errors
    ///
    /// Returns [`ScenePersistenceError::SceneNotEmpty`] if the current scene has
    /// objects, or [`ScenePersistenceError::Restore`] if the snapshot is invalid.
    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError>;
}

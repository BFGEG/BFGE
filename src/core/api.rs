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
///
/// Import this trait to call operations on [`crate::Engine`]. Window operations
/// require a desktop session; scene operations and snapshots also work headlessly.
pub trait EngineApi {
    /// Creates the window and initializes the graphics system.
    ///
    /// Call this on the application's main thread with nonzero window dimensions.
    /// The engine owns the window and its event loop. The scene created by
    /// [`crate::Engine::new`] is preserved, including models loaded beforehand.
    /// Reuse the same engine for later sessions: only one window event loop
    /// may be created per process.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::AlreadyInitialized`] if already initialized,
    /// [`EngineError::Platform`] for invalid dimensions or window failures, or
    /// [`EngineError::Lifecycle`] for graphics initialization failures.
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError>;

    /// Runs the engine's main loop until the window closes or an error occurs.
    ///
    /// The loop calls frame handlers and renders the scene; the client does not
    /// drive individual frames. The engine remains initialized until
    /// [`Self::shutdown`] is called. Every return clears pending events and
    /// retains handlers and scene changes.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInitialized`] before initialization or after
    /// shutdown. Handler errors return as [`EngineError::Scene`]; rendering and
    /// platform errors preserve their causes. A rendering error skips
    /// `frame_ended`.
    fn run(&mut self) -> Result<(), EngineError>;

    /// Registers user frame handlers in registration order.
    ///
    /// All `frame_started` handlers run before frame assembly, then all
    /// `frame_ended` handlers run after a successful render attempt, including
    /// a skipped presentation. A handler error stops the loop; applied changes
    /// remain. Both phases pause while either window dimension is zero.
    ///
    /// Registration is available before initialization; handlers survive
    /// shutdown. Captured scene IDs must be valid before the next run.
    /// Handlers are owned and may capture state, but any captured borrows must
    /// be `'static`. See [`EngineEvent`] for an animation example.
    ///
    /// # Errors
    ///
    /// Registration currently always succeeds. Handler invocation errors are
    /// returned by [`Self::run`].
    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError>;

    /// Releases graphics and the window, and resets the scene and resources.
    ///
    /// The saved snapshot, window event loop and frame handlers are retained.
    /// Old scene IDs are not checked for session identity and may refer to
    /// unrelated nodes after a reset. Call this even after [`Self::run`] fails.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInitialized`] if no session is active.
    fn shutdown(&mut self) -> Result<(), EngineError>;

    /// Returns access to operations over the current scene.
    ///
    /// Available immediately after construction and after shutdown. Changes
    /// made before [`Self::initialize`] are preserved by initialization.
    fn scene(&mut self) -> &mut dyn SceneApi;

    /// Loads a model from `path` and attaches a new object to `parent`.
    ///
    /// Every call creates a new object, including repeated loads of one file.
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

    /// Sets the window size in physical pixels; both dimensions must be nonzero.
    ///
    /// The backend is resized before requesting the window resize. During
    /// [`Self::run`], the caller cannot access the engine; handlers only receive
    /// [`SceneApi`]. Native resize events are handled by the loop automatically.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInitialized`] without a session,
    /// [`EngineError::Platform`] for zero dimensions or a window resize failure,
    /// or [`EngineError::Render`] if the backend rejects the requested size.
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError>;

    /// Saves nodes, objects, mesh data and the viewpoint in memory.
    ///
    /// Replaces the previous snapshot. Survives shutdown, but is lost when the
    /// engine is dropped. Does not save the window or frame handlers, or write
    /// a file. Available without graphics initialization.
    ///
    /// # Errors
    ///
    /// Currently always succeeds; the result allows future storage failures.
    fn save_scene(&mut self) -> Result<(), ScenePersistenceError>;

    /// Replaces an object-free scene and its resources with the saved snapshot.
    ///
    /// Nodes alone do not make a scene occupied. No save is a successful no-op.
    /// The occupancy check precedes the snapshot check. Does not recreate the
    /// window or register frame handlers. Available without initialization.
    ///
    /// # Errors
    ///
    /// Returns [`ScenePersistenceError::SceneNotEmpty`] when objects exist, or
    /// [`ScenePersistenceError::Restore`] for an invalid snapshot. On error the
    /// current scene and resources remain unchanged.
    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError>;
}

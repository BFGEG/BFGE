//! Scene and model operations available during a frame callback.

use std::path::Path;

use crate::platform::{InputState, WindowSize};
use crate::resources::manager::ResourceManager;
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::manager::SceneManager;
use crate::systems::loading::{self, ModelLoadError};

/// Borrows the current scene, resources and input for one frame callback.
///
/// Changes are immediate: start callbacks affect the current frame, while end
/// callbacks affect the next frame. Model loading is synchronous.
pub struct FrameContext<'a> {
    scene: &'a mut SceneManager,
    resources: &'a mut ResourceManager,
    input: &'a InputState,
    window_size: WindowSize,
    exit_requested: &'a mut bool,
}

impl<'a> FrameContext<'a> {
    pub(crate) fn new(
        scene: &'a mut SceneManager,
        resources: &'a mut ResourceManager,
        input: &'a InputState,
        window_size: WindowSize,
        exit_requested: &'a mut bool,
    ) -> Self {
        Self {
            scene,
            resources,
            input,
            window_size,
            exit_requested,
        }
    }

    /// Returns mutable access to the scene for this callback.
    pub fn scene(&mut self) -> &mut dyn SceneApi {
        self.scene
    }

    /// Returns the input collected from window events before this frame.
    ///
    /// The state is read-only: pressed keys and buttons, cursor position and
    /// the mouse motion accumulated before the callback.
    pub fn input(&self) -> &InputState {
        self.input
    }

    /// Requests a successful return from [`crate::EngineApi::run`].
    ///
    /// The current frame completes, including later handlers, rendering and
    /// input reset. Repeated requests have the same effect. A callback or render
    /// error still takes precedence and is returned normally. The session stays
    /// initialized; the caller remains responsible for calling
    /// [`crate::EngineApi::shutdown`]. A later `run` starts without this request.
    pub fn request_exit(&mut self) {
        *self.exit_requested = true;
    }

    /// Returns this frame's rendering area in physical pixels.
    ///
    /// Uses the size applied to rendering after processing resize events.
    /// Cursor positions from [`Self::input`] use the same physical pixel units.
    pub fn window_size(&self) -> WindowSize {
        self.window_size
    }

    /// Loads `path` and attaches a new object to `parent` during the callback.
    ///
    /// Supports the same formats and behavior as [`crate::EngineApi::load_model`].
    /// Each successful call creates a separate mesh resource and object.
    ///
    /// # Errors
    ///
    /// Returns a model loading error without changing the scene or resources.
    pub fn load_model(
        &mut self,
        path: &Path,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, ModelLoadError> {
        loading::load_model(path, self.resources, self.scene, parent)
    }
}

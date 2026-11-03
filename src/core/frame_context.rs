//! Scene and model operations available during a frame callback.

use std::path::Path;

use crate::resources::manager::ResourceManager;
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::manager::SceneManager;
use crate::systems::loading::{self, ModelLoadError};

/// Borrows the current scene and resources for one frame callback.
///
/// Changes are immediate: start callbacks affect the current frame, while end
/// callbacks affect the next frame. Model loading is synchronous.
pub struct FrameContext<'a> {
    scene: &'a mut SceneManager,
    resources: &'a mut ResourceManager,
}

impl<'a> FrameContext<'a> {
    pub(crate) fn new(scene: &'a mut SceneManager, resources: &'a mut ResourceManager) -> Self {
        Self { scene, resources }
    }

    /// Returns mutable access to the scene for this callback.
    pub fn scene(&mut self) -> &mut dyn SceneApi {
        self.scene
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

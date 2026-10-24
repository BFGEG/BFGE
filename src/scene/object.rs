//! Scene object.

use crate::resources::handle::MeshHandle;

/// A scene object: a mesh instance placed in the scene tree.
#[derive(Debug)]
pub struct SceneObject {
    #[allow(dead_code)]
    pub(crate) mesh: MeshHandle,
}

impl SceneObject {
    pub(crate) fn new(mesh: MeshHandle) -> Self {
        Self { mesh }
    }
}

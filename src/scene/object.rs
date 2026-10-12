//! Scene object.

use crate::resources::handle::MeshHandle;

/// A scene object: a mesh instance placed in the scene tree.
#[derive(Debug)]
pub struct SceneObject {
    #[allow(dead_code)] // read by [SCN] build_render_frame (milestone 2)
    pub(crate) mesh: MeshHandle,
}

impl SceneObject {
    pub(crate) fn new(mesh: MeshHandle) -> Self {
        Self { mesh }
    }
}

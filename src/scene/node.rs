//! Scene node.

use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::transform::Transform;

/// A node of the scene tree.
#[derive(Debug, Default)]
pub struct SceneNode {
    #[allow(dead_code)] // read by [SCN] manager (milestone 2)
    pub(crate) parent: Option<SceneNodeId>,
    #[allow(dead_code)] // read by [SCN] manager (milestone 2)
    pub(crate) children: Vec<SceneNodeId>,
    pub(crate) objects: Vec<SceneObjectId>,
    #[allow(dead_code)] // read by [SCN] build_render_frame (milestone 2)
    pub(crate) local_transform: Transform,
}

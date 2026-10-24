//! Scene node.

use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::transform::Transform;

/// A node of the scene tree.
#[derive(Debug, Default)]
pub struct SceneNode {
    #[allow(dead_code)]
    pub(crate) parent: Option<SceneNodeId>,
    #[allow(dead_code)]
    pub(crate) children: Vec<SceneNodeId>,
    pub(crate) objects: Vec<SceneObjectId>,
    #[allow(dead_code)]
    pub(crate) local_transform: Transform,
}

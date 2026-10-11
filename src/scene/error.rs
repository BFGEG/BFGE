//! Scene errors.

use crate::scene::ids::{SceneNodeId, SceneObjectId};

/// Scene error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SceneError {
    /// Unknown scene node.
    #[error("unknown scene node: {0:?}")]
    UnknownNode(SceneNodeId),
    /// Unknown scene object.
    #[error("unknown scene object: {0:?}")]
    UnknownObject(SceneObjectId),
    /// A node cannot be moved into its own descendant.
    #[error("invalid reparent: a node cannot be moved into its own descendant")]
    InvalidReparent,
}

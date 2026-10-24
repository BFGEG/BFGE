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
    /// An invalid reparenting operation reported by a scene callback.
    #[error("invalid reparenting operation")]
    InvalidReparent,
    /// The directional light must have a finite, normalizable direction.
    #[error("light direction must be finite and normalizable")]
    InvalidLightDirection,
    /// Light colors must contain finite, nonnegative components.
    #[error("light colors must be finite and nonnegative")]
    InvalidLightColor,
}

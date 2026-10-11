//! Opaque scene identifiers.

/// Identifier of a scene node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNodeId(pub(crate) u32);

/// Identifier of a scene object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneObjectId(pub(crate) u32);

//! Client-facing scene operations.

use crate::scene::error::SceneError;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::transform::Transform;
use crate::scene::viewpoint::ViewpointState;

/// Operations to create and modify the spatial scene.
pub trait SceneApi {
    /// Returns the root node created together with the scene.
    fn root_node(&self) -> SceneNodeId;

    /// Creates a child node for the given parent node.
    fn create_node(&mut self, parent: SceneNodeId) -> Result<SceneNodeId, SceneError>;

    /// Sets the node's translation, rotation and scale relative to its parent.
    ///
    /// For the root node the transform is relative to the scene.
    fn set_node_transform(
        &mut self,
        node: SceneNodeId,
        transform: Transform,
    ) -> Result<(), SceneError>;

    /// Reparents an object to another node.
    ///
    /// After reparenting, the object's position, rotation and scale are
    /// determined by the new node.
    fn reparent_object(
        &mut self,
        object: SceneObjectId,
        new_parent: SceneNodeId,
    ) -> Result<(), SceneError>;

    /// Returns the current state of the single viewpoint.
    fn viewpoint(&self) -> ViewpointState;

    /// Sets the viewpoint's position, view direction and field of view.
    fn set_viewpoint(&mut self, state: ViewpointState) -> Result<(), SceneError>;
}

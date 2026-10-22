//! Client-facing scene operations.

use crate::scene::error::SceneError;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::lighting::SceneLighting;
use crate::scene::transform::Transform;
use crate::scene::viewpoint::ViewpointState;

/// Operations to create and modify the spatial scene.
///
/// Nodes form a tree; objects loaded through [`crate::EngineApi::load_model`]
/// inherit their node's transform. IDs belong to the current scene and should
/// not be reused after a reset.
///
/// ```
/// use BFGE::math::Vec3;
/// use BFGE::{Engine, EngineApi, SceneError, Transform};
///
/// let mut engine = Engine::new();
/// let scene = engine.scene();
/// let root = scene.root_node();
/// let node = scene.create_node(root)?;
/// scene.set_node_transform(node, Transform::from_translation(Vec3::X))?;
/// # Ok::<(), SceneError>(())
/// ```
pub trait SceneApi {
    /// Returns the root node created together with the scene.
    fn root_node(&self) -> SceneNodeId;

    /// Creates a child node for the given parent node.
    ///
    /// The new node has an identity transform and no objects.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownNode`] if `parent` does not exist.
    fn create_node(&mut self, parent: SceneNodeId) -> Result<SceneNodeId, SceneError>;

    /// Sets the node's translation, rotation and scale relative to its parent.
    ///
    /// For the root node the transform is relative to the scene.
    /// Replaces the entire transform; retain it in client state to update one
    /// component. The world matrix is `parent_world * local`, where local
    /// applies scale, then rotation, then translation. Values are not validated.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownNode`] if `node` does not exist.
    fn set_node_transform(
        &mut self,
        node: SceneNodeId,
        transform: Transform,
    ) -> Result<(), SceneError>;

    /// Reparents an object to another node.
    ///
    /// After reparenting, the object's position, rotation and scale are
    /// determined by the new node.
    /// The previous world transform is not preserved. Other objects on the
    /// old node remain attached to it; this does not reparent a node subtree.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] for an unknown `object`, or
    /// [`SceneError::UnknownNode`] for an unknown `new_parent`, without changes.
    fn reparent_object(
        &mut self,
        object: SceneObjectId,
        new_parent: SceneNodeId,
    ) -> Result<(), SceneError>;

    /// Returns the current state of the single viewpoint.
    ///
    /// Independent of node transforms. Defaults to position `(0, 0, 5)`,
    /// direction `(0, 0, -1)` and a vertical field of view of 60°.
    fn viewpoint(&self) -> ViewpointState;

    /// Sets the viewpoint's position, view direction and field of view.
    ///
    /// The direction is a vector, not a target position. Use finite values,
    /// a nonzero direction not parallel to the world up axis `+Y`, and a
    /// vertical field of view in radians strictly between zero and π.
    /// Values are stored without validation.
    ///
    /// # Errors
    ///
    /// Currently always succeeds; the result allows future validation errors.
    fn set_viewpoint(&mut self, state: ViewpointState) -> Result<(), SceneError>;

    /// Returns the ambient and directional lighting currently used by the scene.
    fn lighting(&self) -> SceneLighting;

    /// Sets the ambient and directional lighting for subsequent frames.
    ///
    /// Colors must be finite and nonnegative. The direction must be finite and
    /// normalizable; it points in the direction the light rays travel.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::InvalidLightDirection`] or
    /// [`SceneError::InvalidLightColor`] for invalid input. The previous
    /// lighting remains active on error.
    fn set_lighting(&mut self, lighting: SceneLighting) -> Result<(), SceneError>;
}

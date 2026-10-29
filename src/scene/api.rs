//! Client-facing scene operations.

use crate::scene::error::SceneError;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::lighting::SceneLighting;
use crate::scene::transform::Transform;
use crate::scene::viewpoint::ViewpointState;

/// Operations to create and modify the scene hierarchy, viewpoint and lighting.
pub trait SceneApi {
    /// Returns the root node created together with the scene.
    ///
    /// # Returns
    ///
    /// The identifier of the scene's root node.
    fn root_node(&self) -> SceneNodeId;

    /// Creates a child node with an identity transform.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownNode`] if `parent` does not identify a node
    /// in this scene.
    ///
    /// # Returns
    ///
    /// The identifier assigned to the new node.
    fn create_node(&mut self, parent: SceneNodeId) -> Result<SceneNodeId, SceneError>;

    /// Returns a copy of the node's local translation, rotation and scale.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownNode`] if `node` does not exist.
    fn node_transform(&self, node: SceneNodeId) -> Result<Transform, SceneError>;

    /// Sets the node's translation, rotation and scale relative to its parent.
    ///
    /// For the root node the transform is relative to the scene. Values are
    /// stored as provided; the operation does not validate finiteness or scale.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownNode`] if `node` does not identify a node
    /// in this scene.
    fn set_node_transform(
        &mut self,
        node: SceneNodeId,
        transform: Transform,
    ) -> Result<(), SceneError>;

    /// Creates an instance of `object` under `parent`, sharing its mesh.
    ///
    /// Visibility and the color override are copied independently. The new
    /// object inherits the target node's transform. No geometry is
    /// copied. The returned identifier is new and is not reused after removal.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] or [`SceneError::UnknownNode`] for
    /// an unknown or removed source or an unknown parent, without changes.
    fn clone_object(
        &mut self,
        object: SceneObjectId,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, SceneError>;

    /// Removes `object` from the scene, leaving its node and shared mesh intact.
    ///
    /// The removed identifier becomes invalid. Other object identifiers and
    /// instances of the mesh are unaffected. Mesh resources live until shutdown.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] for an unknown or removed object,
    /// without changing the scene.
    fn remove_object(&mut self, object: SceneObjectId) -> Result<(), SceneError>;

    /// Sets whether this object is drawn, retaining its ID and mesh.
    ///
    /// Other objects and child nodes remain unaffected. New objects are visible.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] for an unknown or removed object.
    fn set_object_visible(
        &mut self,
        object: SceneObjectId,
        visible: bool,
    ) -> Result<(), SceneError>;

    /// Overrides this object's color with finite, nonnegative linear RGB.
    ///
    /// The mesh and other instances retain their colors. Lighting still applies;
    /// this operation does not enable transparency.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] for an unknown or removed object,
    /// or [`SceneError::InvalidObjectColor`] for invalid RGB, without changes.
    fn set_object_color(
        &mut self,
        object: SceneObjectId,
        color: [f32; 3],
    ) -> Result<(), SceneError>;

    /// Removes this object's color override, restoring the model's original color.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] for an unknown or removed object.
    fn reset_object_color(&mut self, object: SceneObjectId) -> Result<(), SceneError>;

    /// Reparents an object to another node.
    ///
    /// After reparenting, the object's position, rotation and scale are
    /// determined by the new node. Its previous node no longer references it.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::UnknownObject`] or [`SceneError::UnknownNode`] if
    /// either identifier does not belong to this scene. On error, no change is
    /// made.
    fn reparent_object(
        &mut self,
        object: SceneObjectId,
        new_parent: SceneNodeId,
    ) -> Result<(), SceneError>;

    /// Returns the current state of the single viewpoint.
    ///
    /// # Returns
    ///
    /// A copy of the viewpoint state.
    fn viewpoint(&self) -> ViewpointState;

    /// Sets the viewpoint's position, view direction and field of view.
    ///
    /// Values are stored as provided; this operation does not validate them.
    /// Use a finite position, a finite nonzero direction not parallel to the
    /// world up axis, and a field of view between zero and π radians to form a
    /// meaningful view and projection.
    ///
    /// # Errors
    ///
    /// This implementation currently accepts every value and returns `Ok(())`.
    fn set_viewpoint(&mut self, state: ViewpointState) -> Result<(), SceneError>;

    /// Returns the ambient and directional lighting currently used by the scene.
    ///
    /// # Returns
    ///
    /// A copy of the current lighting settings.
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

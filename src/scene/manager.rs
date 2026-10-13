//! Scene storage and management.

use crate::math::{self, Mat4};
use crate::platform::WindowSize;
use crate::render::frame::{RenderCamera, RenderFrame, RenderItem};
use crate::resources::handle::MeshHandle;
use crate::scene::api::SceneApi;
use crate::scene::error::SceneError;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::node::SceneNode;
use crate::scene::object::SceneObject;
use crate::scene::transform::Transform;
use crate::scene::viewpoint::ViewpointState;

/// Manages the scene tree, scene objects and the viewpoint.
#[derive(Debug)]
pub struct SceneManager {
    root: SceneNodeId,
    nodes: Vec<SceneNode>,
    objects: Vec<SceneObject>,
    viewpoint: ViewpointState,
}

impl SceneManager {
    /// Creates a scene with a root node and a default viewpoint.
    pub fn new() -> Self {
        Self {
            root: SceneNodeId(0),
            nodes: vec![SceneNode::new(None)],
            objects: Vec::new(),
            viewpoint: ViewpointState::default(),
        }
    }

    /// Engine-internal operation: creates a scene object.
    ///
    /// Object IDs are monotonic and never reused; this holds because v1 does
    /// not delete nodes or objects (§6.2). The caller must ensure `mesh` refers
    /// to a registered mesh: `SceneManager` cannot validate handles (§29).
    #[allow(dead_code)] // called by [SYS]
    pub(crate) fn create_object(
        &mut self,
        mesh: MeshHandle,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, SceneError> {
        if parent.0 as usize >= self.nodes.len() {
            return Err(SceneError::UnknownNode(parent));
        }

        let id = SceneObjectId(self.objects.len() as u32);
        self.objects.push(SceneObject::new(mesh));
        self.nodes[parent.0 as usize].objects.push(id);

        Ok(id)
    }

    /// Builds a frame from the current scene state.
    ///
    /// The camera comes from the viewpoint and the window size. The scene tree
    /// is traversed from the root, accumulating world transforms, and every
    /// attached object becomes a [`RenderItem`].
    #[allow(dead_code)] // called by [CORE] (milestone 2)
    pub(crate) fn build_render_frame(&self, size: WindowSize) -> RenderFrame {
        let aspect_ratio = if size.height == 0 {
            1.0
        } else {
            size.width as f32 / size.height as f32
        };

        let mut items = Vec::new();
        self.collect_items(self.root, Mat4::IDENTITY, &mut items);

        RenderFrame {
            camera: RenderCamera {
                view: math::view(self.viewpoint.position, self.viewpoint.direction),
                projection: math::perspective(self.viewpoint.fov_y, aspect_ratio),
            },
            items,
        }
    }

    /// Appends a render item for each object in the subtree rooted at `node`,
    /// composing `world = parent_world * local_transform` down the tree.
    fn collect_items(&self, node: SceneNodeId, parent_world: Mat4, items: &mut Vec<RenderItem>) {
        let Some(current) = self.nodes.get(node.0 as usize) else {
            return;
        };

        let world = parent_world * current.local_transform.matrix();

        for object in &current.objects {
            if let Some(object) = self.objects.get(object.0 as usize) {
                items.push(RenderItem {
                    world_transform: world,
                    mesh: object.mesh,
                });
            }
        }

        for child in &current.children {
            self.collect_items(*child, world, items);
        }
    }

    /// Returns all scene nodes.
    pub(crate) fn nodes(&self) -> &[SceneNode] {
        &self.nodes
    }

    /// Returns all scene objects.
    pub(crate) fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    /// Restores nodes, objects and the viewpoint.
    pub(crate) fn restore(
        &mut self,
        nodes: Vec<SceneNode>,
        objects: Vec<SceneObject>,
        viewpoint: ViewpointState,
    ) {
        self.nodes = nodes;
        self.objects = objects;
        self.viewpoint = viewpoint;
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneApi for SceneManager {
    fn root_node(&self) -> SceneNodeId {
        self.root
    }

    fn create_node(&mut self, parent: SceneNodeId) -> Result<SceneNodeId, SceneError> {
        if parent.0 as usize >= self.nodes.len() {
            return Err(SceneError::UnknownNode(parent));
        }

        let id = SceneNodeId(self.nodes.len() as u32);
        self.nodes.push(SceneNode::new(Some(parent)));
        self.nodes[parent.0 as usize].children.push(id);

        Ok(id)
    }

    fn set_node_transform(
        &mut self,
        node: SceneNodeId,
        transform: Transform,
    ) -> Result<(), SceneError> {
        let Some(target) = self.nodes.get_mut(node.0 as usize) else {
            return Err(SceneError::UnknownNode(node));
        };

        target.local_transform = transform;
        Ok(())
    }

    fn reparent_object(
        &mut self,
        object: SceneObjectId,
        new_parent: SceneNodeId,
    ) -> Result<(), SceneError> {
        if self.objects.get(object.0 as usize).is_none() {
            return Err(SceneError::UnknownObject(object));
        }
        if new_parent.0 as usize >= self.nodes.len() {
            return Err(SceneError::UnknownNode(new_parent));
        }

        for node in &mut self.nodes {
            node.objects.retain(|candidate| *candidate != object);
        }
        self.nodes[new_parent.0 as usize].objects.push(object);

        Ok(())
    }

    fn viewpoint(&self) -> ViewpointState {
        self.viewpoint
    }

    fn set_viewpoint(&mut self, state: ViewpointState) -> Result<(), SceneError> {
        self.viewpoint = state;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

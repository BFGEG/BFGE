//! Scene storage and management.

use crate::platform::WindowSize;
use crate::render::frame::RenderFrame;
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
    #[allow(dead_code)] // called by [SYS]
    pub(crate) fn create_object(
        &mut self,
        _mesh: MeshHandle,
        _parent: SceneNodeId,
    ) -> Result<SceneObjectId, SceneError> {
        todo!("milestone 2")
    }

    /// Builds a frame: tree traversal, world transforms, camera from the
    /// viewpoint and the image size.
    #[allow(dead_code)] // called by [CORE]
    pub(crate) fn build_render_frame(&self, _size: WindowSize) -> RenderFrame {
        todo!("milestone 2")
    }

    /// Returns all scene nodes.
    #[allow(dead_code)] // read by [SYS]
    pub(crate) fn nodes(&self) -> &[SceneNode] {
        &self.nodes
    }

    /// Returns all scene objects.
    #[allow(dead_code)] // read by [SYS]
    pub(crate) fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    /// Restores nodes, objects and the viewpoint.
    #[allow(dead_code)] // called by [SYS]
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

    fn create_node(&mut self, _parent: SceneNodeId) -> Result<SceneNodeId, SceneError> {
        todo!("milestone 2")
    }

    fn set_node_transform(
        &mut self,
        _node: SceneNodeId,
        _transform: Transform,
    ) -> Result<(), SceneError> {
        todo!("milestone 2")
    }

    fn reparent_object(
        &mut self,
        _object: SceneObjectId,
        _new_parent: SceneNodeId,
    ) -> Result<(), SceneError> {
        todo!("milestone 2")
    }

    fn viewpoint(&self) -> ViewpointState {
        self.viewpoint
    }

    fn set_viewpoint(&mut self, state: ViewpointState) -> Result<(), SceneError> {
        self.viewpoint = state;
        Ok(())
    }
}

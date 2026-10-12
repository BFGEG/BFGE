//! Scene storage and management.

use crate::math;
use crate::platform::WindowSize;
use crate::render::frame::{RenderCamera, RenderFrame};
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

    /// Builds a frame from the current scene state.
    ///
    /// The camera comes from the viewpoint and the window size. Object
    /// collection and world transforms are added in the next milestone, so
    /// `items` is currently empty.
    #[allow(dead_code)] // called by [CORE] (milestone 2)
    pub(crate) fn build_render_frame(&self, size: WindowSize) -> RenderFrame {
        let aspect_ratio = if size.height == 0 {
            1.0
        } else {
            size.width as f32 / size.height as f32
        };

        RenderFrame {
            camera: RenderCamera {
                view: math::view(self.viewpoint.position, self.viewpoint.direction),
                projection: math::perspective(self.viewpoint.fov_y, aspect_ratio),
            },
            items: Vec::new(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;
    use crate::scene::api::SceneApi;

    const EPSILON: f32 = 1e-5;
    const WINDOW: WindowSize = WindowSize {
        width: 800,
        height: 600,
    };

    #[test]
    fn frame_camera_is_placed_at_the_viewpoint() {
        let mut scene = SceneManager::new();
        let position = Vec3::new(1.0, 2.0, 3.0);
        let viewpoint = ViewpointState {
            position,
            direction: Vec3::new(0.0, 0.0, -1.0),
            fov_y: 60.0_f32.to_radians(),
        };
        scene
            .set_viewpoint(viewpoint)
            .expect("the scene accepts a valid viewpoint");

        let frame = scene.build_render_frame(WINDOW);

        let origin = frame.camera.view.transform_point3(position);
        assert!(origin.abs_diff_eq(Vec3::ZERO, EPSILON));
    }

    #[test]
    fn zero_height_window_produces_finite_projection() {
        let scene = SceneManager::new();

        let frame = scene.build_render_frame(WindowSize::new(800, 0));

        assert!(frame.camera.projection.is_finite());
    }
}

//! Data boundary between the scene and the renderer.

use crate::math::Mat4;
use crate::resources::handle::MeshHandle;
use crate::scene::lighting::SceneLighting;

/// Scene state prepared for one rendering pass.
#[derive(Debug, Clone)]
pub struct RenderFrame {
    /// View and projection matrices for the frame.
    pub camera: RenderCamera,
    /// Ambient and directional lighting for the frame.
    pub lighting: SceneLighting,
    /// Objects to draw, each with its scene transform and mesh identifier.
    pub items: Vec<RenderItem>,
}

/// Camera data in matrix form, ready for the rendering backend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderCamera {
    /// View matrix (world -> view).
    pub view: Mat4,
    /// Projection matrix (view -> clip).
    pub projection: Mat4,
}

/// One scene object prepared for drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderItem {
    /// Optional linear RGB override for this instance; otherwise use the mesh color.
    pub color_override: Option<[f32; 3]>,
    /// Object transform in scene coordinates.
    pub world_transform: Mat4,
    /// Identifier of the object's geometry resource.
    pub mesh: MeshHandle,
}

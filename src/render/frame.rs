//! Data boundary between the scene and the renderer.

use crate::math::Mat4;
use crate::resources::handle::MeshHandle;

/// Display frame - the data boundary between the scene and the renderer.
#[derive(Debug, Clone)]
pub struct RenderFrame {
    /// Frame camera.
    pub camera: RenderCamera,
    /// Frame items.
    pub items: Vec<RenderItem>,
}

/// Frame camera: ready view/projection matrices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderCamera {
    /// View matrix (world -> view).
    pub view: Mat4,
    /// Projection matrix (view -> clip).
    pub projection: Mat4,
}

/// A single frame item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderItem {
    /// World transform matrix.
    pub world_transform: Mat4,
    /// Object's geometry.
    pub mesh: MeshHandle,
}

//! Viewpoint state.

use crate::math::Vec3;

/// Position, direction and perspective field of view of the scene viewpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewpointState {
    /// Viewpoint position in scene coordinates.
    pub position: Vec3,
    /// Direction in which the viewpoint looks; it is used as provided.
    pub direction: Vec3,
    /// Vertical field of view in radians; it is used as provided.
    pub fov_y: f32,
}

impl Default for ViewpointState {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
            fov_y: 60.0_f32.to_radians(),
        }
    }
}

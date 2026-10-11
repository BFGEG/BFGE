//! Viewpoint state.

use crate::math::Vec3;

/// State of the single viewpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewpointState {
    /// Viewpoint position.
    pub position: Vec3,
    /// View direction.
    pub direction: Vec3,
    /// Vertical field of view, radians.
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

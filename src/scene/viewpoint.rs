//! Viewpoint state.

use crate::math::Vec3;

/// State of the single viewpoint.
///
/// Uses a right-handed coordinate system with `+Y` up, independently of scene
/// nodes. Defaults to `(0, 0, 5)`, looking along `-Z`, with a 60° vertical field
/// of view. Smaller `fov_y` values zoom in. The current projection clips at
/// 0.1 and 100 world units; the aspect ratio follows the window size.
/// See [`crate::SceneApi::set_viewpoint`] for numeric preconditions.
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

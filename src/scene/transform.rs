//! Node transform: translation, rotation and scale.

use crate::math::{Mat4, Quat, Vec3};

/// Position, quaternion rotation and scale relative to the parent node.
///
/// The local matrix applies scale, then rotation, then translation. Defaults
/// to zero translation, identity rotation and unit scale. Setting a node's
/// transform replaces all three components; constructors set the other
/// components to their defaults. Numeric values are not validated by the scene.
///
/// ```
/// use BFGE::math::{Quat, Vec3};
/// use BFGE::Transform;
///
/// let transform = Transform {
///     translation: Vec3::new(1.0, 0.0, 0.0),
///     rotation: Quat::from_rotation_y(0.5),
///     scale: Vec3::splat(0.5),
/// };
/// assert_eq!(transform.translation.x, 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Translation in parent-space units.
    pub translation: Vec3,
    /// Rotation as a unit quaternion.
    pub rotation: Quat,
    /// Independent scale factors for the X, Y and Z axes.
    pub scale: Vec3,
}

impl Transform {
    /// Returns the identity transform.
    pub fn identity() -> Self {
        Self::default()
    }

    /// Creates a transform from a translation only.
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform from a rotation only.
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform from Euler angles (radians, XYZ order).
    pub fn from_euler(euler_radians: Vec3) -> Self {
        Self::from_rotation(Quat::from_euler(
            glam::EulerRot::XYZ,
            euler_radians.x,
            euler_radians.y,
            euler_radians.z,
        ))
    }

    /// Creates a transform from a scale only.
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
        }
    }

    /// Local transform matrix.
    #[allow(dead_code)] // used by [SCN] build_render_frame (milestone 2)
    pub(crate) fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn matrix_translates_a_point() {
        let transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));

        let translated = transform.matrix().transform_point3(Vec3::ZERO);

        assert!(translated.abs_diff_eq(Vec3::new(1.0, 2.0, 3.0), EPSILON));
    }
}

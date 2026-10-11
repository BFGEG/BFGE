//! Node transform: translation, rotation and scale.

use crate::math::{Mat4, Quat, Vec3};

/// Position, rotation and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Translation.
    pub translation: Vec3,
    /// Rotation.
    pub rotation: Quat,
    /// Scale.
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

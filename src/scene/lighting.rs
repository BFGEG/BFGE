//! Lighting shared by every object in a scene.

use crate::math::Vec3;
use crate::scene::error::SceneError;

/// A directional light whose rays travel along `direction` in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    /// Direction in which light rays travel; it need not be normalized.
    pub direction: Vec3,
    /// Linear RGB intensity of the directional light.
    pub color: Vec3,
}

/// Constant ambient light and one directional light for the whole scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneLighting {
    /// Linear RGB ambient intensity applied to every surface.
    pub ambient: Vec3,
    /// The scene's directional light.
    pub directional: DirectionalLight,
}

impl SceneLighting {
    pub(crate) fn validate(self) -> Result<(), SceneError> {
        if self.directional.direction.try_normalize().is_none() {
            Err(SceneError::InvalidLightDirection)
        } else if !valid_color(self.ambient) || !valid_color(self.directional.color) {
            Err(SceneError::InvalidLightColor)
        } else {
            Ok(())
        }
    }
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            ambient: Vec3::splat(0.25),
            directional: DirectionalLight {
                direction: Vec3::new(-0.5, -1.0, -0.5),
                color: Vec3::splat(0.75),
            },
        }
    }
}

fn valid_color(color: Vec3) -> bool {
    color.is_finite() && color.min_element() >= 0.0
}

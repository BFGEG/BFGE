//! Scene object.

use crate::resources::handle::MeshHandle;
use crate::scene::error::SceneError;

/// A scene object: a mesh instance placed in the scene tree.
#[derive(Debug)]
pub struct SceneObject {
    #[allow(dead_code)]
    pub(crate) mesh: MeshHandle,
    pub(crate) visible: bool,
    pub(crate) color_override: Option<[f32; 3]>,
}

impl SceneObject {
    pub(crate) fn new(mesh: MeshHandle) -> Self {
        Self {
            mesh,
            visible: true,
            color_override: None,
        }
    }
}

pub(crate) fn validate_color(color: [f32; 3]) -> Result<(), SceneError> {
    if color.iter().all(|value| value.is_finite() && *value >= 0.0) {
        Ok(())
    } else {
        Err(SceneError::InvalidObjectColor)
    }
}

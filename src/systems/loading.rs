//! Model loading from files.

use std::path::{Path, PathBuf};

use crate::resources::manager::ResourceManager;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::manager::SceneManager;

/// Model loading error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModelLoadError {
    /// Unsupported model format.
    #[error("unsupported model format: {0}")]
    UnsupportedFormat(String),
    /// Failed to read the model file.
    #[error("failed to read model file {path}: {source}")]
    Io {
        /// Path to the model file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse the model file.
    #[error("failed to parse model file {path}: {message}")]
    Parse {
        /// Path to the model file.
        path: PathBuf,
        /// Parse error message.
        message: String,
    },
}

/// Loads a model (glTF 2.0 / OBJ) and creates a scene object for it.
///
/// # Errors
///
/// Returns [`ModelLoadError`] for an unsupported or corrupted file.
pub fn load_model(
    _path: &Path,
    _resources: &mut ResourceManager,
    _scene: &mut SceneManager,
    _parent: SceneNodeId,
) -> Result<SceneObjectId, ModelLoadError> {
    todo!("glTF/OBJ loading: milestone 2")
}

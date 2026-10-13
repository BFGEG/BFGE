//! Model loading from files.

use std::path::{Path, PathBuf};

use crate::resources::manager::ResourceManager;
use crate::resources::mesh::Mesh;
use crate::scene::error::SceneError;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::manager::SceneManager;

mod gltf;
mod obj;

const GLTF_EXTENSION: &str = "gltf";
const GLB_EXTENSION: &str = "glb";
const OBJ_EXTENSION: &str = "obj";
const DEFAULT_MODEL_COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];

type LoadResult<T> = std::result::Result<T, ModelLoadError>;

/// Model loading error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModelLoadError {
    /// Failed to attach the model to the requested scene node.
    #[error("failed to attach model: {0}")]
    Scene(#[from] SceneError),
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
/// Returns [`ModelLoadError`] for an unknown parent, unsupported format,
/// unreadable file or invalid geometry. Errors leave resources and scene intact.
pub fn load_model(
    path: &Path,
    resources: &mut ResourceManager,
    scene: &mut SceneManager,
    parent: SceneNodeId,
) -> Result<SceneObjectId, ModelLoadError> {
    if scene.nodes().get(parent.0 as usize).is_none() {
        Err(SceneError::UnknownNode(parent).into())
    } else {
        let mesh = read_mesh(path)?;
        let handle = resources.add_mesh(mesh);
        scene.create_object(handle, parent).map_err(Into::into)
    }
}

fn read_mesh(path: &Path) -> LoadResult<Mesh> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mesh = match extension.as_str() {
        GLTF_EXTENSION | GLB_EXTENSION => gltf::load(path)?,
        OBJ_EXTENSION => obj::load(path)?,
        _ => return Err(ModelLoadError::UnsupportedFormat(extension)),
    };
    validate_mesh(path, &mesh)?;
    Ok(mesh)
}

fn read_file(path: &Path) -> LoadResult<Vec<u8>> {
    std::fs::read(path).map_err(|source| ModelLoadError::Io {
        path: path.to_owned(),
        source,
    })
}

fn parse_error(path: &Path, message: impl ToString) -> ModelLoadError {
    ModelLoadError::Parse {
        path: path.to_owned(),
        message: message.to_string(),
    }
}

fn validate_mesh(path: &Path, mesh: &Mesh) -> LoadResult<()> {
    if mesh.submeshes.is_empty() {
        return Err(parse_error(path, "model contains no triangle geometry"));
    }
    if mesh
        .color
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(parse_error(
            path,
            "model color must contain finite values in [0, 1]",
        ));
    }
    for (index, part) in mesh.submeshes.iter().enumerate() {
        if part.vertices.is_empty()
            || part.indices.is_empty()
            || !part.indices.len().is_multiple_of(3)
            || part
                .vertices
                .iter()
                .any(|vertex| !vertex.position.is_finite())
            || part
                .indices
                .iter()
                .any(|&index| index as usize >= part.vertices.len())
        {
            return Err(parse_error(
                path,
                format!("invalid triangle geometry in part {index}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

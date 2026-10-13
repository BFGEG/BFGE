//! Storage and management of shared geometry resources.

use crate::resources::handle::MeshHandle;
use crate::resources::library::{BuiltinMesh, MeshLibrary};
use crate::resources::mesh::Mesh;

/// Manager of shared geometry resources.
#[derive(Debug, Default)]
pub struct ResourceManager {
    meshes: Vec<Mesh>,
    library: MeshLibrary,
}

impl ResourceManager {
    /// Creates an empty resource manager.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_mesh(&mut self, mesh: Mesh) -> MeshHandle {
        let index = self.meshes.len() as u32;
        self.meshes.push(mesh);
        MeshHandle::new(index)
    }

    /// Returns the handle of a built-in `shape`, registering it on first use.
    #[allow(dead_code)] // called by [SYS]
    pub(crate) fn builtin(&mut self, shape: BuiltinMesh) -> MeshHandle {
        if let Some(handle) = self.library.handle(shape) {
            handle
        } else {
            let handle = self.add_mesh(MeshLibrary::build(shape));
            self.library.register(shape, handle);
            handle
        }
    }

    pub(crate) fn mesh(&self, handle: MeshHandle) -> Option<&Mesh> {
        self.meshes.get(handle.index() as usize)
    }

    #[allow(dead_code)] // read by [SYS]
    pub(crate) fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    #[allow(dead_code)] // used by [SYS]
    pub(crate) fn restore_meshes(&mut self, meshes: Vec<Mesh>) {
        self.meshes = meshes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_cube_is_registered_once() {
        let mut resources = ResourceManager::new();

        let first = resources.builtin(BuiltinMesh::Cube);
        let second = resources.builtin(BuiltinMesh::Cube);

        assert_eq!(first, second);
        assert_eq!(resources.meshes().len(), 1);
        assert!(resources.mesh(first).is_some());
    }
}

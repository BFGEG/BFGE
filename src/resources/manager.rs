//! Storage and management of shared geometry resources.

use crate::resources::handle::MeshHandle;
use crate::resources::library::MeshLibrary;
use crate::resources::mesh::Mesh;

/// Manager of shared geometry resources.
#[derive(Debug, Default)]
pub struct ResourceManager {
    meshes: Vec<Mesh>,
    #[allow(dead_code)] // populated by [RES]-1
    library: MeshLibrary,
}

impl ResourceManager {
    /// Creates an empty resource manager.
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)] // populated by [SYS]
    pub(crate) fn add_mesh(&mut self, mesh: Mesh) -> MeshHandle {
        let index = self.meshes.len() as u32;
        self.meshes.push(mesh);
        MeshHandle::new(index)
    }

    #[allow(dead_code)] // read by [BKD]/[SCN]
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

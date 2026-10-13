//! GPU vertex and index buffers for the wgpu backend.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bytemuck::{Pod, Zeroable};

use crate::math::Vec3;
use crate::render::system::RenderError;
use crate::resources::handle::MeshHandle;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::Mesh;

const VERTEX_BUFFER_LABEL: &str = "BFGE mesh vertices";
const INDEX_BUFFER_LABEL: &str = "BFGE mesh indices";

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub(crate) position: [f32; 3],
    pub(crate) tint: [f32; 3],
}

impl GpuVertex {
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<GpuVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };

    fn from_position(position: Vec3) -> Self {
        Self {
            position: position.to_array(),
            tint: tint(position),
        }
    }
}

fn tint(position: Vec3) -> [f32; 3] {
    let value = 0.5 + 0.5 * (position.x + position.y + position.z) / 3.0;
    let value = value.clamp(0.0, 1.0);
    [value, value, value]
}

#[derive(Debug)]
pub(crate) struct GpuMesh {
    pub(crate) submeshes: Vec<GpuSubMesh>,
    pub(crate) color: [f32; 4],
}

#[derive(Debug)]
pub(crate) struct GpuSubMesh {
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    pub(crate) index_count: u32,
}

#[derive(Debug, Default)]
pub(crate) struct MeshBufferCache {
    meshes: HashMap<MeshHandle, GpuMesh>,
}

impl MeshBufferCache {
    pub(crate) fn get(
        &mut self,
        device: &wgpu::Device,
        resources: &ResourceManager,
        handle: MeshHandle,
    ) -> Result<&GpuMesh, RenderError> {
        match self.meshes.entry(handle) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let mesh = resources.mesh(handle).ok_or_else(|| {
                    RenderError::Render(format!("unknown mesh handle {handle:?}"))
                })?;
                Ok(entry.insert(upload_mesh(device, mesh)))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.meshes.len()
    }
}

fn upload_mesh(device: &wgpu::Device, mesh: &Mesh) -> GpuMesh {
    let submeshes = mesh
        .submeshes
        .iter()
        .map(|submesh| {
            let vertices = submesh
                .vertices
                .iter()
                .map(|vertex| GpuVertex::from_position(vertex.position))
                .collect::<Vec<_>>();
            let vertices = create_buffer(
                device,
                VERTEX_BUFFER_LABEL,
                bytemuck::cast_slice(&vertices),
                wgpu::BufferUsages::VERTEX,
            );
            let indices = create_buffer(
                device,
                INDEX_BUFFER_LABEL,
                bytemuck::cast_slice(&submesh.indices),
                wgpu::BufferUsages::INDEX,
            );
            GpuSubMesh {
                vertices,
                indices,
                index_count: submesh.indices.len() as u32,
            }
        })
        .collect();
    GpuMesh {
        submeshes,
        color: mesh.color,
    }
}

fn create_buffer(
    device: &wgpu::Device,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents,
        usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_is_clamped_to_unit_range() {
        assert_eq!(tint(Vec3::new(-100.0, -100.0, -100.0)), [0.0, 0.0, 0.0]);
        assert_eq!(tint(Vec3::new(100.0, 100.0, 100.0)), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn tint_is_grayscale_and_deterministic() {
        let a = tint(Vec3::new(0.5, -0.5, 0.5));
        assert_eq!(a[0], a[1]);
        assert_eq!(a[1], a[2]);
        assert_eq!(a, tint(Vec3::new(0.5, -0.5, 0.5)));
    }

    #[test]
    fn vertex_layout_offsets_match_the_struct() {
        let attributes = GpuVertex::LAYOUT.attributes;
        assert_eq!(GpuVertex::LAYOUT.array_stride, 24);
        assert_eq!(attributes[0].offset, 0);
        assert_eq!(attributes[1].offset, 12);
        assert_eq!(attributes[0].shader_location, 0);
        assert_eq!(attributes[1].shader_location, 1);
    }

    #[test]
    fn cache_len_starts_empty() {
        assert_eq!(MeshBufferCache::default().len(), 0);
    }
}

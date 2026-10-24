//! GPU vertex and index buffers for the wgpu backend.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bytemuck::{Pod, Zeroable};

use crate::math::Vec3;
use crate::render::system::RenderError;
use crate::resources::handle::MeshHandle;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::{Mesh, SubMesh};

const VERTEX_BUFFER_LABEL: &str = "BFGE mesh vertices";
const INDEX_BUFFER_LABEL: &str = "BFGE mesh indices";

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
}

impl GpuVertex {
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<GpuVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };

    fn from_position_and_normal(position: Vec3, normal: Vec3) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
        }
    }
}

fn vertex_normals(submesh: &SubMesh) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; submesh.vertices.len()];
    for triangle in submesh.indices.chunks_exact(3) {
        let [a, b, c] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let (Some(va), Some(vb), Some(vc)) = (
            submesh.vertices.get(a),
            submesh.vertices.get(b),
            submesh.vertices.get(c),
        ) else {
            continue;
        };
        let normal = (vb.position - va.position).cross(vc.position - va.position);
        for index in [a, b, c] {
            normals[index] += normal;
        }
    }
    normals.into_iter().map(Vec3::normalize_or_zero).collect()
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
            let normals = vertex_normals(submesh);
            let vertices = submesh
                .vertices
                .iter()
                .zip(normals)
                .map(|(vertex, normal)| {
                    GpuVertex::from_position_and_normal(vertex.position, normal)
                })
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
    fn cube_normals_point_outward() {
        use crate::resources::library::{BuiltinMesh, MeshLibrary};

        let cube = MeshLibrary::build(BuiltinMesh::Cube);
        let submesh = &cube.submeshes[0];
        let normals = vertex_normals(submesh);

        for (vertex, normal) in submesh.vertices.iter().zip(normals) {
            assert!(normal.dot(vertex.position) > 0.0);
        }
    }

    #[test]
    fn degenerate_triangle_has_zero_normals() {
        use crate::resources::mesh::Vertex;

        let submesh = SubMesh {
            vertices: vec![
                Vertex {
                    position: Vec3::ZERO,
                },
                Vertex { position: Vec3::X },
                Vertex {
                    position: Vec3::X * 2.0,
                },
            ],
            indices: vec![0, 1, 2],
        };

        assert_eq!(vertex_normals(&submesh), vec![Vec3::ZERO; 3]);
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

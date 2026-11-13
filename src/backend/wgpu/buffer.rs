//! GPU vertex and index buffers for the wgpu backend.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use image::GenericImageView; 
use bytemuck::{Pod, Zeroable};

use crate::math::{Vec3, Vec2};
use crate::render::system::RenderError;
use crate::resources::handle::{MeshHandle};
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::{Mesh, SubMesh, Material};

const VERTEX_BUFFER_LABEL: &str = "BFGE mesh vertices";
const INDEX_BUFFER_LABEL: &str = "BFGE mesh indices";

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) uv: [f32; 2],
}


impl GpuVertex {
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<GpuVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
    };

    fn from_position_and_normal(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            uv: uv.to_array(),
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
pub(crate) struct GpuMaterial {
    pub(crate) name: String,
    pub(crate) diffuse_texture: wgpu::Texture,
    pub(crate) bind_group: wgpu::BindGroup,
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
    pub(crate) material: usize,
}

#[derive(Debug, Default)]
pub(crate) struct MeshCache {
    meshes: HashMap<MeshHandle, GpuMesh>,
}

#[derive(Debug, Default)]
pub(crate) struct MaterialCache {
    materials: HashMap<usize, GpuMaterial>,
}

impl MeshCache {
    pub(crate) fn get_mesh(
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

impl MaterialCache {
    pub(crate) fn get_material(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &ResourceManager,
        handle: MeshHandle,
        id: usize,
    ) -> Result<&GpuMaterial, RenderError> {
        match self.materials.entry(id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let mesh = resources.mesh(handle).ok_or_else(|| {
                    RenderError::Render(format!("unknown mesh handle {handle:?}"))
                })?;
                let material = mesh.materials.get(id).ok_or_else(|| {
                    RenderError::Render(format!("unknown material handle {id:?}"))
                })?;
                Ok(entry.insert(upload_material(device, queue, material)))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.materials.len()
    }
}

fn upload_material(device: &wgpu::Device, queue: &wgpu::Queue, material: &Material) -> GpuMaterial {
    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture_bind_group_layout"),
        entries: &[

            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
                  wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    match material {
        Material::ObjMaterial(tobj) => {
            let path: String = tobj.path.to_string_lossy().into_owned();
            let img = image::open(path).expect("Failed to load diffuse image");
            let rgba = img.to_rgba8();
            let (width, height) = img.dimensions();

            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&tobj.material.name),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &diffuse_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                size,
            );

            let view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some(&format!("{}_bind_group", tobj.material.name)),
            });
        GpuMaterial {
            name : tobj.material.name.to_string(),
            diffuse_texture,
            bind_group
        }
        }
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
                    GpuVertex::from_position_and_normal(vertex.position, normal, vertex.uv)
                })
                .collect::<Vec<_>>();
            let vertex_buffer = create_buffer(
                device,
                VERTEX_BUFFER_LABEL,
                bytemuck::cast_slice(&vertices),
                wgpu::BufferUsages::VERTEX,
            );
            let index_buffer = create_buffer(
                device,
                INDEX_BUFFER_LABEL,
                bytemuck::cast_slice(&submesh.indices),
                wgpu::BufferUsages::INDEX,
            );
            GpuSubMesh {
                vertices : vertex_buffer,
                indices :index_buffer,
                index_count: submesh.indices.len() as u32,
                material: submesh.material,
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
        use crate::math::Vec2;
        use crate::resources::mesh::Vertex;

        let submesh = SubMesh {
            vertices: vec![
                Vertex {
                    position: Vec3::ZERO,
                    uv: Vec2::ZERO,
                },
                Vertex {
                    position: Vec3::X,
                    uv: Vec2::ZERO,
                },
                Vertex {
                    position: Vec3::X * 2.0,
                    uv: Vec2::ZERO,
                },
            ],
            indices: vec![0, 1, 2],
            material: 0,
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
        assert_eq!(MeshCache::default().len(), 0);
    }
}

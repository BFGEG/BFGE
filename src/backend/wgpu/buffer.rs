//! GPU vertex and index buffers for the wgpu backend.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bytemuck::{Pod, Zeroable};
use image::GenericImageView;

use crate::math::{Vec2, Vec3};
use crate::render::system::RenderError;
use crate::resources::handle::MeshHandle;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::{Material, Mesh, SubMesh};

const VERTEX_BUFFER_LABEL: &str = "BFGE mesh vertices";
const INDEX_BUFFER_LABEL: &str = "BFGE mesh indices";
const FALLBACK_MATERIAL_LABEL: &str = "BFGE fallback material";
const WHITE_PIXEL: [u8; 4] = [255, 255, 255, 255];

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
/// WGPU-specific structure for materials.
pub struct GpuMaterial {
    /// Texture bind group.
    pub bind_group: wgpu::BindGroup,
    /// Material opacity, if supplied by the model.
    pub opacity: Option<f32>,
    /// Whether the diffuse texture contains non-opaque pixels.
    pub texture_transparent: bool,
}

/// WGPU-specific structure for meshes.
#[derive(Debug)]
pub struct GpuMesh {
    /// Vector of submeshes.
    pub submeshes: Vec<GpuSubMesh>,
    /// Default color of mesh in case of materials absence.
    pub color: [f32; 4],
}

#[derive(Debug)]
/// WGPU-specific structure for submeshes.
pub struct GpuSubMesh {
    /// WGPU vertex buffer.
    pub vertex_buffer: wgpu::Buffer,
    /// WGPU index buffer.
    pub index_buffer: wgpu::Buffer,
    /// Index count.
    pub index_count: u32,
    /// Material identificator.
    pub material: usize,
    /// Model-space bounds center used to sort transparent parts.
    pub center: Vec3,
}

#[derive(Debug, Default)]
/// Cache meshes in hashmap.
pub struct MeshCache {
    /// Hashmap of WGPU meshhes.
    meshes: HashMap<MeshHandle, GpuMesh>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
/// A pair of mesh handle and material identificator.
struct MaterialHandle(MeshHandle, usize);

#[derive(Debug, Default)]
/// Cache materials in hashmap.
pub struct MaterialCache {
    /// Hashmap of WGPU materials.
    materials: HashMap<MaterialHandle, GpuMaterial>,
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
        layout: &wgpu::BindGroupLayout,
        handle: MeshHandle,
        id: usize,
    ) -> Result<&GpuMaterial, RenderError> {
        match self.materials.entry(MaterialHandle(handle, id)) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let mesh = resources.mesh(handle).ok_or_else(|| {
                    RenderError::Render(format!("unknown mesh handle {handle:?}"))
                })?;
                let material = match mesh.materials.get(id) {
                    Some(material) => upload_material(device, queue, material, layout),
                    None => upload_fallback_material(device, queue, layout),
                };
                Ok(entry.insert(material))
            }
        }
    }
}

pub(crate) fn normalize_opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn bounds_center(submesh: &SubMesh) -> Vec3 {
    let Some(first) = submesh.vertices.first() else {
        return Vec3::ZERO;
    };
    let (min, max) = submesh
        .vertices
        .iter()
        .fold((first.position, first.position), |(min, max), vertex| {
            (min.min(vertex.position), max.max(vertex.position))
        });
    min * 0.5 + max * 0.5
}

fn upload_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material: &Material,
    layout: &wgpu::BindGroupLayout,
) -> GpuMaterial {
    let mut texture_transparent = false;
    let (name, texture, opacity) = match material {
        Material::ObjMaterial(tobj) => {
            let name = tobj.material.name.clone();
            let texture = tobj
                .material
                .diffuse_texture
                .as_deref()
                .and_then(|file| tobj.path.parent().map(|dir| dir.join(file)))
                .and_then(|path| image::open(path).ok())
                .map(|image| {
                    let rgba = image.to_rgba8();
                    texture_transparent = rgba.pixels().any(|pixel| pixel[3] < 255);
                    let (width, height) = image.dimensions();
                    create_diffuse_texture(device, queue, width, height, &rgba, &name)
                })
                .unwrap_or_else(|| white_texture(device, queue));
            (
                name,
                texture,
                Some(normalize_opacity(tobj.material.dissolve.unwrap_or(1.0))),
            )
        }
    };
    let bind_group = create_texture_bind_group(device, layout, &texture, &name);
    GpuMaterial {
        bind_group,
        opacity,
        texture_transparent,
    }
}

fn upload_fallback_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> GpuMaterial {
    let texture = white_texture(device, queue);
    let bind_group = create_texture_bind_group(device, layout, &texture, FALLBACK_MATERIAL_LABEL);
    GpuMaterial {
        bind_group,
        opacity: None,
        texture_transparent: false,
    }
}

fn white_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    create_diffuse_texture(device, queue, 1, 1, &WHITE_PIXEL, FALLBACK_MATERIAL_LABEL)
}

fn create_diffuse_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
    label: &str,
) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );
    texture
}

fn create_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    label: &str,
) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
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
    })
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
                vertex_buffer,
                index_buffer,
                index_count: submesh.indices.len() as u32,
                material: submesh.material,
                center: bounds_center(submesh),
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
    fn opacity_is_bounded_and_nonfinite_values_remain_opaque() {
        for (value, expected) in [
            (-1.0, 0.0),
            (0.0, 0.0),
            (0.4, 0.4),
            (2.0, 1.0),
            (f32::NAN, 1.0),
            (f32::INFINITY, 1.0),
        ] {
            assert_eq!(normalize_opacity(value), expected);
        }
    }

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
        assert_eq!(GpuVertex::LAYOUT.array_stride, 32);
        assert_eq!(attributes[0].offset, 0);
        assert_eq!(attributes[1].offset, 12);
        assert_eq!(attributes[2].offset, 24);
        assert_eq!(attributes[0].shader_location, 0);
        assert_eq!(attributes[1].shader_location, 1);
        assert_eq!(attributes[2].shader_location, 2);
    }

    #[test]
    fn cache_len_starts_empty() {
        assert_eq!(MeshCache::default().len(), 0);
    }
}

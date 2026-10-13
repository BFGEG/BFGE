//! CPU-side mesh data prepared for GPU buffers.

use crate::render::system::RenderError;
use crate::resources::handle::MeshHandle;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::Mesh;

use super::checked_gpu_operation;

const VERTEX_BUFFER_LABEL: &str = "BFGE mesh vertices";
const INDEX_BUFFER_LABEL: &str = "BFGE mesh indices";
const INVALID_BUFFER_SIZE: &str = "GPU buffer data must have a nonzero 4-byte-aligned size";
const VERTEX_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl GpuVertex {
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedSubMesh<'a> {
    pub vertices: Vec<GpuVertex>,
    pub indices: &'a [u32],
}

#[derive(Debug)]
pub(crate) struct PreparedMesh<'a> {
    pub submeshes: Vec<PreparedSubMesh<'a>>,
    pub color: [f32; 4],
}

#[derive(Debug)]
pub(crate) struct GpuSubMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

#[derive(Debug)]
pub(crate) struct GpuMesh {
    pub submeshes: Vec<GpuSubMesh>,
    pub color: [f32; 4],
}

impl GpuMesh {
    pub(crate) fn upload(
        device: &wgpu::Device,
        resources: &ResourceManager,
        handle: MeshHandle,
    ) -> Result<Self, RenderError> {
        let prepared = prepare_mesh(resources, handle)?;
        let mut submeshes = Vec::with_capacity(prepared.submeshes.len());

        for (part_index, part) in prepared.submeshes.iter().enumerate() {
            let vertex_buffer = upload_buffer(
                device,
                VERTEX_BUFFER_LABEL,
                bytemuck::cast_slice(&part.vertices),
                wgpu::BufferUsages::VERTEX,
            )
            .map_err(|error| {
                RenderError::Render(format!(
                    "upload vertices of submesh {part_index} in {handle:?}: {error}"
                ))
            })?;
            let index_buffer = upload_buffer(
                device,
                INDEX_BUFFER_LABEL,
                bytemuck::cast_slice(part.indices),
                wgpu::BufferUsages::INDEX,
            )
            .map_err(|error| {
                RenderError::Render(format!(
                    "upload indices of submesh {part_index} in {handle:?}: {error}"
                ))
            })?;
            let index_count = u32::try_from(part.indices.len()).map_err(|error| {
                RenderError::Render(format!(
                    "index count of submesh {part_index} in {handle:?}: {error}"
                ))
            })?;
            submeshes.push(GpuSubMesh {
                vertex_buffer,
                index_buffer,
                index_count,
            });
        }

        Ok(Self {
            submeshes,
            color: prepared.color,
        })
    }
}

pub(crate) fn upload_buffer(
    device: &wgpu::Device,
    label: &'static str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> Result<wgpu::Buffer, RenderError> {
    let size = u64::try_from(contents.len()).map_err(|error| {
        RenderError::Render(format!("{label} size does not fit a GPU buffer: {error}"))
    })?;
    if size == 0 || size % wgpu::COPY_BUFFER_ALIGNMENT != 0 {
        Err(RenderError::Render(INVALID_BUFFER_SIZE.to_owned()))
    } else if size > device.limits().max_buffer_size {
        Err(RenderError::Render(format!(
            "{label} size {size} exceeds GPU buffer limit {}",
            device.limits().max_buffer_size
        )))
    } else {
        checked_gpu_operation(device, || {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: true,
            });
            {
                let mut mapping = buffer.get_mapped_range_mut(..).map_err(|error| {
                    RenderError::Render(format!("map {label} for upload: {error}"))
                })?;
                mapping.slice(..).copy_from_slice(contents);
            }
            buffer.unmap();
            Ok(buffer)
        })?
    }
}

pub(crate) fn prepare_mesh(
    resources: &ResourceManager,
    handle: MeshHandle,
) -> Result<PreparedMesh<'_>, RenderError> {
    let mesh = resources.mesh(handle).ok_or_else(|| {
        RenderError::Render(format!(
            "mesh handle {handle:?} does not reference a resource"
        ))
    })?;
    validate_mesh(handle, mesh)?;

    let submeshes = mesh
        .submeshes
        .iter()
        .map(|part| PreparedSubMesh {
            vertices: part
                .vertices
                .iter()
                .map(|vertex| GpuVertex {
                    position: vertex.position.to_array(),
                    color: VERTEX_COLOR,
                })
                .collect(),
            indices: &part.indices,
        })
        .collect();

    Ok(PreparedMesh {
        submeshes,
        color: mesh.color,
    })
}

fn validate_mesh(handle: MeshHandle, mesh: &Mesh) -> Result<(), RenderError> {
    if mesh.submeshes.is_empty() {
        return Err(invalid_mesh(handle, "no submeshes"));
    }
    if mesh
        .color
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(invalid_mesh(
            handle,
            "color must be finite and within [0, 1]",
        ));
    }

    for (part_index, part) in mesh.submeshes.iter().enumerate() {
        if part.vertices.is_empty() || u32::try_from(part.vertices.len()).is_err() {
            return Err(invalid_mesh(
                handle,
                format!("submesh {part_index} has an invalid vertex count"),
            ));
        }
        if part.indices.is_empty()
            || !part.indices.len().is_multiple_of(3)
            || u32::try_from(part.indices.len()).is_err()
        {
            return Err(invalid_mesh(
                handle,
                format!("submesh {part_index} has an invalid triangle index count"),
            ));
        }
        if part
            .vertices
            .iter()
            .any(|vertex| !vertex.position.is_finite())
        {
            return Err(invalid_mesh(
                handle,
                format!("submesh {part_index} contains a non-finite position"),
            ));
        }
        if part
            .indices
            .iter()
            .any(|&index| index as usize >= part.vertices.len())
        {
            return Err(invalid_mesh(
                handle,
                format!("submesh {part_index} contains an out-of-range index"),
            ));
        }
    }

    Ok(())
}

fn invalid_mesh(handle: MeshHandle, reason: impl std::fmt::Display) -> RenderError {
    RenderError::Render(format!("invalid mesh {handle:?}: {reason}"))
}

#[cfg(test)]
mod tests {
    use crate::math::Vec3;
    use crate::resources::mesh::{SubMesh, Vertex};

    use super::*;

    const MODEL_COLOR: [f32; 4] = [0.8, 0.6, 0.4, 1.0];

    fn triangle() -> SubMesh {
        SubMesh {
            vertices: vec![
                Vertex {
                    position: Vec3::ZERO,
                },
                Vertex { position: Vec3::X },
                Vertex { position: Vec3::Y },
            ],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn gpu_vertex_matches_shader_layout() {
        let layout = GpuVertex::layout();

        assert_eq!(layout.array_stride, 24);
        assert_eq!(layout.attributes[0].shader_location, 0);
        assert_eq!(layout.attributes[0].offset, 0);
        assert_eq!(layout.attributes[0].format, wgpu::VertexFormat::Float32x3);
        assert_eq!(layout.attributes[1].shader_location, 1);
        assert_eq!(layout.attributes[1].offset, 12);
        assert_eq!(layout.attributes[1].format, wgpu::VertexFormat::Float32x3);
    }

    #[test]
    fn preparation_preserves_submeshes_and_model_color() {
        let mut resources = ResourceManager::new();
        let handle = resources.add_mesh(Mesh {
            submeshes: vec![triangle(), triangle()],
            color: MODEL_COLOR,
        });

        let prepared = prepare_mesh(&resources, handle).expect("both submeshes are valid");

        assert_eq!(prepared.submeshes.len(), 2);
        assert_eq!(prepared.color, MODEL_COLOR);
        assert_eq!(prepared.submeshes[0].indices, &[0, 1, 2]);
        assert_eq!(prepared.submeshes[0].vertices[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(prepared.submeshes[0].vertices[1].color, VERTEX_COLOR);
    }

    #[test]
    fn unknown_handle_returns_render_error() {
        let resources = ResourceManager::new();

        assert!(matches!(
            prepare_mesh(&resources, MeshHandle::new(7)),
            Err(RenderError::Render(message)) if message.contains("MeshHandle(7)")
        ));
    }

    #[test]
    fn out_of_range_index_returns_render_error() {
        let mut resources = ResourceManager::new();
        let mut part = triangle();
        part.indices[2] = 3;
        let handle = resources.add_mesh(Mesh {
            submeshes: vec![part],
            color: MODEL_COLOR,
        });

        assert!(matches!(
            prepare_mesh(&resources, handle),
            Err(RenderError::Render(message)) if message.contains("out-of-range index")
        ));
    }

    #[test]
    fn non_finite_position_returns_render_error() {
        let mut resources = ResourceManager::new();
        let mut part = triangle();
        part.vertices[0].position.x = f32::NAN;
        let handle = resources.add_mesh(Mesh {
            submeshes: vec![part],
            color: MODEL_COLOR,
        });

        assert!(matches!(
            prepare_mesh(&resources, handle),
            Err(RenderError::Render(message)) if message.contains("non-finite position")
        ));
    }

    #[test]
    fn malformed_triangle_indices_return_render_error() {
        let mut resources = ResourceManager::new();
        let mut part = triangle();
        part.indices.pop();
        let handle = resources.add_mesh(Mesh {
            submeshes: vec![part],
            color: MODEL_COLOR,
        });

        assert!(matches!(
            prepare_mesh(&resources, handle),
            Err(RenderError::Render(message)) if message.contains("triangle index count")
        ));
    }

    #[test]
    fn empty_mesh_returns_render_error() {
        let mut resources = ResourceManager::new();
        let handle = resources.add_mesh(Mesh {
            submeshes: Vec::new(),
            color: MODEL_COLOR,
        });

        assert!(matches!(
            prepare_mesh(&resources, handle),
            Err(RenderError::Render(message)) if message.contains("no submeshes")
        ));
    }

    #[test]
    fn invalid_color_returns_render_error() {
        let mut resources = ResourceManager::new();
        let handle = resources.add_mesh(Mesh {
            submeshes: vec![triangle()],
            color: [f32::NAN, 0.0, 0.0, 1.0],
        });

        assert!(matches!(
            prepare_mesh(&resources, handle),
            Err(RenderError::Render(message)) if message.contains("color")
        ));
    }
}

//! CPU-side mesh data prepared for GPU buffers.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "GPU upload follows in issue #28")
)]

use crate::render::system::RenderError;
use crate::resources::handle::MeshHandle;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::Mesh;

const VERTEX_COLOR: [f32; 3] = [1.0, 1.0, 1.0];

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GpuVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
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
        assert_eq!(std::mem::size_of::<GpuVertex>(), 24);
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

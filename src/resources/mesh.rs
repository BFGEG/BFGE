//! Geometry: vertices, mesh parts, and the mesh itself.

use crate::math::Vec3;

/// Geometry vertex. In version 1 - position only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Vertex position.
    pub position: Vec3,
}

/// Part of geometry: vertices and indices.
#[derive(Debug, Clone, PartialEq)]
pub struct SubMesh {
    /// Vertices of the part.
    pub vertices: Vec<Vertex>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

/// Geometry resource and the model's common simple color.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Geometry parts.
    pub submeshes: Vec<SubMesh>,
    /// Common color of the model, linear RGBA.
    pub color: [f32; 4],
}

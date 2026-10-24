//! Geometry: vertices, mesh parts, and the mesh itself.

use crate::math::Vec3;

/// A geometry vertex containing a position in model space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Vertex position.
    pub position: Vec3,
}

/// Indexed triangle geometry forming one part of a mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct SubMesh {
    /// Vertices of the part.
    pub vertices: Vec<Vertex>,
    /// Indices into `vertices`, grouped in triples to form triangles.
    pub indices: Vec<u32>,
}

/// Geometry resource and the single linear RGBA color used for all its parts.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Geometry parts.
    pub submeshes: Vec<SubMesh>,
    /// Common color of the model, linear RGBA.
    pub color: [f32; 4],
}

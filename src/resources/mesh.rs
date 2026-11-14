//! Geometry: vertices, mesh parts, and the mesh itself.

use crate::math::{Vec2, Vec3};
use std::path::PathBuf;

/// A geometry vertex containing a position in model space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Vertex position.
    pub position: Vec3,
    /// UV Map position.
    pub uv: Vec2,
}

/// Indexed triangle geometry forming one part of a mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct SubMesh {
    /// Vertices of the part.
    pub vertices: Vec<Vertex>,
    /// Indices into `vertices`, grouped in triples to form triangles.
    pub indices: Vec<u32>,
    /// Material identificator.
    pub material: usize,
}

/// A wrapper for tobj::material.
#[derive(Debug, Clone)]
pub struct TobjMaterial {
    /// Material from tobj library.
    pub material: tobj::Material,
    /// Path to .obj file.
    pub path: PathBuf,
}

impl PartialEq for TobjMaterial {
    fn eq(&self, other: &Self) -> bool {
        let a = &self.material;
        let b = &other.material;
        self.path == other.path
            && a.name == b.name
            && a.ambient == b.ambient
            && a.diffuse == b.diffuse
            && a.specular == b.specular
            && a.emissive == b.emissive
            && a.shininess == b.shininess
            && a.dissolve == b.dissolve
            && a.optical_density == b.optical_density
            && a.ambient_texture == b.ambient_texture
            && a.diffuse_texture == b.diffuse_texture
            && a.specular_texture == b.specular_texture
            && a.normal_texture == b.normal_texture
            && a.shininess_texture == b.shininess_texture
            && a.dissolve_texture == b.dissolve_texture
            && a.illumination_model == b.illumination_model
            && a.unknown_param == b.unknown_param
    }
}

/// Enum of materials.
#[derive(Debug, Clone, PartialEq)]
pub enum Material {
    /// The .mtl material for .obj
    ObjMaterial(TobjMaterial),
}

/// Geometry resource and the single linear RGBA color used for all its parts.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Geometry parts.
    pub submeshes: Vec<SubMesh>,
    /// Common color of the model, linear RGBA.
    pub color: [f32; 4],
    /// Material enum.
    pub materials: Vec<Material>,
}

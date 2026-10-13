//! Library of built-in (procedural) meshes.

use std::collections::HashMap;

use crate::math::Vec3;
use crate::resources::handle::MeshHandle;
use crate::resources::mesh::{Mesh, SubMesh, Vertex};

/// Linear RGBA color of the built-in cube. Displays as CSS `lightgray`
/// (#D3D3D3) on an sRGB swapchain.
const CUBE_COLOR: [f32; 4] = [0.65, 0.65, 0.65, 1.0];

/// Corner positions of the six cube faces, in face order +X, -X, +Y, -Y, +Z,
/// -Z. Each row lists the face's four corners in counter-clockwise order as
/// seen from outside the cube, so the triangle split per face
/// (`[b, b+1, b+2]`, `[b, b+2, b+3]` with `b = face * 4`) winds outward.
const CUBE_FACES: [[Vec3; 4]; 6] = [
    [
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(0.5, 0.5, 0.5),
    ],
    [
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, -0.5),
    ],
    [
        Vec3::new(-0.5, 0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
    ],
    [
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(-0.5, -0.5, 0.5),
    ],
    [
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
    ],
    [
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
    ],
];

/// Identifier of a built-in (procedural) mesh.
#[allow(dead_code)] // no non-test constructor until [SYS] wiring lands; tests construct it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BuiltinMesh {
    /// A unit cube centered at the origin.
    Cube,
}

/// Library of built-in (procedural) meshes.
///
/// Supplies ready-made geometry (currently a unit cube) as a temporary source
/// of meshes until file loading (`load_model`) is implemented. In v1 the only
/// client path for creating scene objects is `load_model`; creating objects
/// from built-in meshes is a post-v1 feature (§13.3).
#[derive(Debug, Default)]
pub struct MeshLibrary {
    meshes: HashMap<BuiltinMesh, MeshHandle>,
}

impl MeshLibrary {
    /// Returns the handle of an already registered built-in `shape`.
    pub(crate) fn handle(&self, shape: BuiltinMesh) -> Option<MeshHandle> {
        self.meshes.get(&shape).copied()
    }

    /// Records the handle of a registered built-in `shape`.
    pub(crate) fn register(&mut self, shape: BuiltinMesh, handle: MeshHandle) {
        self.meshes.insert(shape, handle);
    }

    /// Builds the geometry of a built-in `shape` procedurally.
    pub(crate) fn build(shape: BuiltinMesh) -> Mesh {
        match shape {
            BuiltinMesh::Cube => Self::cube(),
        }
    }

    /// Builds a unit cube centered at the origin with outward-facing winding.
    #[allow(dead_code)]
    fn cube() -> Mesh {
        let mut vertices = Vec::with_capacity(CUBE_FACES.len() * 4);
        let mut indices = Vec::with_capacity(CUBE_FACES.len() * 6);
        for (face, corners) in CUBE_FACES.iter().enumerate() {
            let base = (face * 4) as u32;
            for corner in corners {
                vertices.push(Vertex { position: *corner });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        Mesh {
            submeshes: vec![SubMesh { vertices, indices }],
            color: CUBE_COLOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn cube() -> Mesh {
        MeshLibrary::build(BuiltinMesh::Cube)
    }

    fn cube_submesh() -> SubMesh {
        cube()
            .submeshes
            .into_iter()
            .next()
            .expect("cube has one submesh")
    }

    #[test]
    fn cube_has_24_vertices_and_36_indices() {
        let submesh = cube_submesh();

        assert_eq!(submesh.vertices.len(), 24);
        assert_eq!(submesh.indices.len(), 36);
    }

    #[test]
    fn cube_indices_are_in_range() {
        let submesh = cube_submesh();

        assert_eq!(submesh.indices.len() % 3, 0);
        assert!(
            submesh
                .indices
                .iter()
                .all(|&index| (index as usize) < submesh.vertices.len())
        );
    }

    #[test]
    fn cube_has_eight_unique_positions() {
        let submesh = cube_submesh();

        let unique: HashSet<[u32; 3]> = submesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.to_array().map(f32::to_bits))
            .collect();

        assert_eq!(unique.len(), 8);
    }

    #[test]
    fn cube_triangles_face_outward() {
        let submesh = cube_submesh();

        for triangle in submesh.indices.chunks_exact(3) {
            let a = submesh.vertices[triangle[0] as usize].position;
            let b = submesh.vertices[triangle[1] as usize].position;
            let c = submesh.vertices[triangle[2] as usize].position;

            let normal = (b - a).cross(c - a);
            let centroid = (a + b + c) / 3.0;

            assert!(
                normal.dot(centroid) > EPSILON,
                "triangle faces inward: {triangle:?}"
            );
        }
    }

    #[test]
    fn cube_occupies_unit_bounds() {
        let submesh = cube_submesh();

        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(-f32::INFINITY);
        for vertex in &submesh.vertices {
            min = min.min(vertex.position);
            max = max.max(vertex.position);
        }

        assert!(min.abs_diff_eq(Vec3::splat(-0.5), EPSILON));
        assert!(max.abs_diff_eq(Vec3::splat(0.5), EPSILON));
    }
}

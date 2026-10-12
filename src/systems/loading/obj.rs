use std::cell::RefCell;
use std::path::Path;

use crate::math::Vec3;
use crate::resources::mesh::{Mesh, SubMesh, Vertex};

use super::{DEFAULT_MODEL_COLOR, ModelLoadError, parse_error, read_file};

const POLYGON_TOLERANCE: f32 = 1e-5;

pub(super) fn load(path: &Path) -> Result<Mesh, ModelLoadError> {
    let bytes = read_file(path)?;
    let material_error = RefCell::new(None);
    let parsed = tobj::load_obj_buf(
        &mut bytes.as_slice(),
        &tobj::LoadOptions::default(),
        |relative| {
            let directory = path.parent().unwrap_or_else(|| Path::new(""));
            let material_path = directory.join(relative);
            match read_file(&material_path).and_then(|bytes| {
                tobj::load_mtl_buf(&mut bytes.as_slice())
                    .map_err(|error| parse_error(&material_path, error))
            }) {
                Ok(materials) => Ok(materials),
                Err(error) => {
                    material_error.borrow_mut().get_or_insert(error);
                    Err(tobj::LoadError::ReadError)
                }
            }
        },
    );
    if let Some(error) = material_error.into_inner() {
        return Err(error);
    }
    let (models, materials) = parsed.map_err(|error| parse_error(path, error))?;
    let materials = materials.map_err(|error| parse_error(path, error))?;
    let mut color = None;
    let mut parts = Vec::new();
    for model in models {
        let mesh = model.mesh;
        if mesh.indices.is_empty() {
            continue;
        }
        if color.is_none() {
            color = mesh
                .material_id
                .and_then(|index| materials.get(index))
                .and_then(|material| {
                    material
                        .diffuse
                        .map(|[r, g, b]| [r, g, b, material.dissolve.unwrap_or(1.0)])
                });
        }
        if !mesh.positions.len().is_multiple_of(3) {
            return Err(parse_error(path, "OBJ positions must contain XYZ triples"));
        }
        let vertices = mesh
            .positions
            .chunks_exact(3)
            .map(|position| Vertex {
                position: Vec3::new(position[0], position[1], position[2]),
            })
            .collect::<Vec<_>>();
        if vertices.iter().any(|vertex| !vertex.position.is_finite())
            || mesh
                .indices
                .iter()
                .any(|&index| index as usize >= vertices.len())
        {
            return Err(parse_error(
                path,
                "OBJ contains non-finite positions or out-of-range indices",
            ));
        }
        let indices = triangulate(path, mesh.indices, &mesh.face_arities, &vertices)?;
        parts.push(SubMesh { vertices, indices });
    }
    Ok(Mesh {
        submeshes: parts,
        color: color.unwrap_or(DEFAULT_MODEL_COLOR),
    })
}

fn triangulate(
    path: &Path,
    indices: Vec<u32>,
    arities: &[u32],
    vertices: &[Vertex],
) -> Result<Vec<u32>, ModelLoadError> {
    if arities.is_empty() {
        Ok(indices)
    } else {
        let mut remaining = indices.as_slice();
        let mut triangles = Vec::new();
        for &arity in arities {
            let count = arity as usize;
            let face = remaining
                .get(..count)
                .ok_or_else(|| parse_error(path, "OBJ face exceeds its index buffer"))?;
            if count < 3 {
                return Err(parse_error(path, "OBJ points and lines are unsupported"));
            }
            if count > 3 {
                validate_polygon(path, face, vertices)?;
            }
            triangles.extend(
                face[1..]
                    .windows(2)
                    .flat_map(|edge| [face[0], edge[0], edge[1]]),
            );
            remaining = &remaining[count..];
        }
        if remaining.is_empty() {
            Ok(triangles)
        } else {
            Err(parse_error(
                path,
                "OBJ face sizes do not match the index buffer",
            ))
        }
    }
}

fn validate_polygon(path: &Path, face: &[u32], vertices: &[Vertex]) -> Result<(), ModelLoadError> {
    let positions = face
        .iter()
        .map(|&index| vertices[index as usize].position)
        .collect::<Vec<_>>();
    let origin = positions[0];
    let extent = positions
        .iter()
        .map(|&point| (point - origin).length())
        .fold(0.0_f32, f32::max);
    let normal = positions[1..]
        .windows(2)
        .map(|edge| (edge[0] - origin).cross(edge[1] - origin))
        .find_map(Vec3::try_normalize)
        .ok_or_else(|| parse_error(path, "OBJ polygon is degenerate"))?;
    if !extent.is_finite()
        || positions
            .iter()
            .any(|&point| (point - origin).dot(normal).abs() > extent * POLYGON_TOLERANCE)
    {
        return Err(parse_error(path, "OBJ polygons must be planar"));
    }
    for index in 0..positions.len() {
        let start = positions[index];
        let end = positions[(index + 1) % positions.len()];
        if start == end
            || positions.iter().any(|&point| {
                (end - start).cross(point - start).dot(normal)
                    < -extent * extent * POLYGON_TOLERANCE
            })
        {
            return Err(parse_error(path, "OBJ polygons must be simple and convex"));
        }
    }
    Ok(())
}

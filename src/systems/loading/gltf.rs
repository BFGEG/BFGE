use std::path::Path;

use base64::Engine as _;
use gltf::accessor::{DataType, Dimensions};
use gltf::buffer::Source;
use gltf::mesh::Mode;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

use crate::math::{Mat4, Vec3};
use crate::resources::mesh::{Mesh, SubMesh, Vertex};

use super::{DEFAULT_MODEL_COLOR, ModelLoadError, parse_error, read_file};

const DATA_URI_PREFIX: &str = "data:";
const BASE64_SUFFIX: &str = ";base64";
const OCTET_STREAM: &str = "application/octet-stream";
const GLTF_BUFFER: &str = "application/gltf-buffer";
const GLB_MAGIC: &[u8] = b"glTF";

#[derive(Deserialize)]
struct MaterialColors {
    #[serde(default)]
    materials: Vec<MaterialColor>,
}

#[derive(Deserialize)]
struct MaterialColor {
    #[serde(rename = "pbrMetallicRoughness")]
    pbr: Option<PbrColor>,
}

#[derive(Deserialize)]
struct PbrColor {
    #[serde(rename = "baseColorFactor")]
    color: Option<[f32; 4]>,
}

pub(super) fn load(path: &Path) -> Result<Mesh, ModelLoadError> {
    let file = read_file(path)?;
    let model = gltf::Gltf::from_slice(&file).map_err(|error| parse_error(path, error))?;
    let colors: MaterialColors = if file.starts_with(GLB_MAGIC) {
        let binary = gltf::Glb::from_slice(&file).map_err(|error| parse_error(path, error))?;
        serde_json::from_slice(&binary.json)
    } else {
        serde_json::from_slice(&file)
    }
    .map_err(|error| parse_error(path, error))?;
    if model.document.extensions_required().next().is_some() {
        return Err(parse_error(
            path,
            "required glTF extensions are unsupported",
        ));
    }
    let mut buffers = vec![None; model.buffers().len()];
    let scene = model
        .default_scene()
        .or_else(|| model.scenes().next())
        .ok_or_else(|| parse_error(path, "glTF contains no scene"))?;
    let mut pending = scene
        .nodes()
        .map(|node| (node, Mat4::IDENTITY))
        .collect::<Vec<_>>();
    pending.reverse();
    let mut visited = vec![false; model.nodes().len()];
    let mut parts = Vec::new();
    let mut color = None;
    while let Some((node, parent_transform)) = pending.pop() {
        if visited[node.index()] {
            return Err(parse_error(
                path,
                "glTF node hierarchy contains a cycle or repeated node",
            ));
        }
        visited[node.index()] = true;
        if node.skin().is_some() {
            return Err(parse_error(path, "skinned glTF geometry is unsupported"));
        }
        let transform = parent_transform * Mat4::from_cols_array_2d(&node.transform().matrix());
        if !transform.is_finite()
            || transform.x_axis.w != 0.0
            || transform.y_axis.w != 0.0
            || transform.z_axis.w != 0.0
            || transform.w_axis.w != 1.0
        {
            return Err(parse_error(
                path,
                "glTF node transform must be finite and affine",
            ));
        }
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                parts.push(load_primitive(
                    path,
                    &primitive,
                    &mut buffers,
                    model.blob.as_deref(),
                    transform,
                )?);
                if color.is_none() {
                    color = primitive
                        .material()
                        .index()
                        .and_then(|index| colors.materials.get(index))
                        .and_then(|material| material.pbr.as_ref())
                        .and_then(|pbr| pbr.color);
                }
            }
        }
        let children = node.children().collect::<Vec<_>>();
        pending.extend(children.into_iter().rev().map(|child| (child, transform)));
    }
    Ok(Mesh {
        submeshes: parts,
        color: color.unwrap_or(DEFAULT_MODEL_COLOR),
    })
}

fn load_buffer(
    path: &Path,
    buffer: &gltf::Buffer<'_>,
    blob: Option<&[u8]>,
) -> Result<Vec<u8>, ModelLoadError> {
    let data = match buffer.source() {
        Source::Bin => blob
            .ok_or_else(|| parse_error(path, "GLB is missing its binary buffer"))?
            .to_vec(),
        Source::Uri(uri) => {
            if let Some(data) = uri.strip_prefix(DATA_URI_PREFIX) {
                let (header, encoded) = data
                    .split_once(',')
                    .ok_or_else(|| parse_error(path, "invalid buffer data URI"))?;
                let media_type = header
                    .strip_suffix(BASE64_SUFFIX)
                    .ok_or_else(|| parse_error(path, "buffer data URI must use base64"))?;
                if !matches!(media_type, OCTET_STREAM | GLTF_BUFFER) {
                    return Err(parse_error(path, "unsupported buffer data URI media type"));
                }
                base64::engine::general_purpose::STANDARD
                    .decode(percent_decode_str(encoded).collect::<Vec<_>>())
                    .map_err(|error| parse_error(path, error))?
            } else {
                if uri.contains(':') || uri.starts_with("//") {
                    return Err(parse_error(
                        path,
                        "only relative local buffer URIs are supported",
                    ));
                }
                let decoded = percent_decode_str(uri)
                    .decode_utf8()
                    .map_err(|error| parse_error(path, error))?;
                let relative = Path::new(decoded.as_ref());
                if relative.is_absolute() {
                    return Err(parse_error(path, "buffer URI must be relative"));
                }
                let directory = path.parent().unwrap_or_else(|| Path::new(""));
                read_file(&directory.join(relative))?
            }
        }
    };
    if data.len() < buffer.length() {
        Err(parse_error(
            path,
            format!(
                "buffer {} is truncated: expected {} bytes, found {}",
                buffer.index(),
                buffer.length(),
                data.len()
            ),
        ))
    } else {
        Ok(data)
    }
}

fn load_primitive(
    path: &Path,
    primitive: &gltf::Primitive<'_>,
    buffers: &mut [Option<Vec<u8>>],
    blob: Option<&[u8]>,
    transform: Mat4,
) -> Result<SubMesh, ModelLoadError> {
    if primitive.mode() != Mode::Triangles || primitive.morph_targets().next().is_some() {
        return Err(parse_error(
            path,
            "only static glTF triangle primitives are supported",
        ));
    }
    let positions = primitive
        .get(&gltf::Semantic::Positions)
        .ok_or_else(|| parse_error(path, "glTF primitive is missing POSITION"))?;
    if positions.data_type() != DataType::F32
        || positions.dimensions() != Dimensions::Vec3
        || positions.normalized()
    {
        return Err(parse_error(
            path,
            "POSITION must be an unnormalized FLOAT VEC3 accessor",
        ));
    }
    validate_accessor(path, &positions, buffers, blob)?;
    if let Some(indices) = primitive.indices() {
        if indices.dimensions() != Dimensions::Scalar
            || indices.normalized()
            || !matches!(
                indices.data_type(),
                DataType::U8 | DataType::U16 | DataType::U32
            )
        {
            return Err(parse_error(
                path,
                "indices must be an unsigned scalar accessor",
            ));
        }
        validate_accessor(path, &indices, buffers, blob)?;
    }
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).and_then(Option::as_deref));
    let vertices = reader
        .read_positions()
        .ok_or_else(|| parse_error(path, "cannot read glTF positions"))?
        .map(|position| Vertex {
            position: transform.transform_point3(Vec3::from_array(position)),
        })
        .collect::<Vec<_>>();
    let mut indices = if let Some(indices) = reader.read_indices() {
        indices.into_u32().collect()
    } else {
        let count = u32::try_from(vertices.len()).map_err(|error| parse_error(path, error))?;
        (0..count).collect::<Vec<_>>()
    };
    if transform.determinant() < 0.0 {
        for triangle in indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }
    Ok(SubMesh { vertices, indices })
}

fn validate_accessor(
    path: &Path,
    accessor: &gltf::Accessor<'_>,
    buffers: &mut [Option<Vec<u8>>],
    blob: Option<&[u8]>,
) -> Result<(), ModelLoadError> {
    if accessor.sparse().is_some() {
        return Err(parse_error(path, "sparse glTF accessors are unsupported"));
    }
    let view = accessor
        .view()
        .ok_or_else(|| parse_error(path, "accessor has no buffer view"))?;
    let stride = view.stride().unwrap_or_else(|| accessor.size());
    let end = accessor
        .count()
        .checked_sub(1)
        .and_then(|count| count.checked_mul(stride))
        .and_then(|length| length.checked_add(accessor.size()))
        .and_then(|length| length.checked_add(accessor.offset()));
    let view_end = view.offset().checked_add(view.length());
    let buffer = buffers
        .get_mut(view.buffer().index())
        .ok_or_else(|| parse_error(path, "missing accessor buffer"))?;
    if buffer.is_none() {
        *buffer = Some(load_buffer(path, &view.buffer(), blob)?);
    }
    let buffer_length = buffer.as_ref().map_or(0, Vec::len);
    if stride < accessor.size()
        || !stride.is_multiple_of(accessor.data_type().size())
        || view
            .offset()
            .checked_add(accessor.offset())
            .is_none_or(|offset| !offset.is_multiple_of(accessor.data_type().size()))
        || !accessor
            .offset()
            .is_multiple_of(accessor.data_type().size())
        || end.is_none_or(|end| end > view.length())
        || view_end.is_none_or(|end| end > view.buffer().length() || end > buffer_length)
    {
        Err(parse_error(
            path,
            format!(
                "accessor {} exceeds its buffer view or has invalid stride/alignment",
                accessor.index()
            ),
        ))
    } else {
        Ok(())
    }
}

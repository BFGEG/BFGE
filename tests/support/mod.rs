use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde_json::{Value, json};
use tempfile::TempDir;

pub const GLTF_FILE: &str = "model.gltf";
pub const GLB_FILE: &str = "model.glb";
pub const BUFFER_FILE: &str = "geometry data.bin";
pub const BUFFER_URI: &str = "geometry%20data.bin";

pub fn directory() -> TempDir {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/model-tests");
    std::fs::create_dir_all(&parent).expect("the repository target directory is writable");
    tempfile::tempdir_in(parent).expect("the repository target directory accepts test fixtures")
}

pub fn triangle_buffer() -> Vec<u8> {
    [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .chain([0_u16, 1, 2].into_iter().flat_map(u16::to_le_bytes))
        .collect()
}

pub fn gltf_document() -> Value {
    json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 }, "indices": 1 }] }],
        "buffers": [{ "byteLength": 42, "uri": BUFFER_URI }],
        "bufferViews": [
            { "buffer": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 1, 0] },
            { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
        ]
    })
}

pub fn write_gltf(directory: &Path, document: &Value, buffer: &[u8]) -> PathBuf {
    let path = directory.join(GLTF_FILE);
    std::fs::write(
        &path,
        serde_json::to_vec(document).expect("fixture JSON is serializable"),
    )
    .expect("the fixture directory is writable");
    std::fs::write(directory.join(BUFFER_FILE), buffer).expect("the fixture directory is writable");
    path
}

pub fn embed_buffer(document: &mut Value, buffer: &[u8]) {
    document["buffers"][0]["uri"] = json!(format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buffer)
    ));
}

pub fn write_glb(directory: &Path, mut document: Value, buffer: &[u8]) -> PathBuf {
    document["buffers"][0]
        .as_object_mut()
        .expect("buffer is a JSON object")
        .remove("uri");
    let mut json = serde_json::to_vec(&document).expect("fixture JSON is serializable");
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut binary = buffer.to_vec();
    while !binary.len().is_multiple_of(4) {
        binary.push(0);
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"glTF");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&((28 + json.len() + binary.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(&(json.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"JSON");
    bytes.extend(json);
    bytes.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"BIN\0");
    bytes.extend(binary);
    let path = directory.join(GLB_FILE);
    std::fs::write(&path, bytes).expect("the fixture directory is writable");
    path
}

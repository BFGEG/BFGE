use std::path::Path;

use BFGE::{Engine, EngineApi, ModelLoadError};

mod support;

use support::*;

#[test]
fn public_api_loads_gltf_and_glb_without_a_graphics_window() {
    let directory = directory();
    let gltf = write_gltf(directory.path(), &gltf_document(), &triangle_buffer());
    let glb = write_glb(directory.path(), gltf_document(), &triangle_buffer());
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    let first = engine
        .load_model(&gltf, root)
        .expect("valid glTF loads through the facade");
    let second = engine
        .load_model(&glb, root)
        .expect("valid GLB loads through the facade");

    assert_ne!(first, second);
}

#[test]
fn public_api_repeated_loads_create_distinct_object_ids() {
    let directory = directory();
    let mut document = gltf_document();
    embed_buffer(&mut document, &triangle_buffer());
    let path = write_gltf(directory.path(), &document, &[]);
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    let first = engine.load_model(&path, root).expect("the model is valid");
    let second = engine
        .load_model(&path, root)
        .expect("the same model may be loaded again");

    assert_ne!(first, second);
}

#[test]
fn public_api_reports_unsupported_format() {
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    assert!(matches!(
        engine.load_model(Path::new("model.fbx"), root),
        Err(ModelLoadError::UnsupportedFormat(_))
    ));
}

#[test]
fn public_api_preserves_io_error_path_and_cause() {
    let directory = directory();
    let path = directory.path().join(GLTF_FILE);
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    assert!(
        matches!(engine.load_model(&path, root), Err(ModelLoadError::Io { path: failed, source })
        if failed == path && source.kind() == std::io::ErrorKind::NotFound)
    );
}

#[test]
fn public_api_reports_corrupt_data_and_can_retry() {
    let directory = directory();
    let path = write_gltf(directory.path(), &gltf_document(), &triangle_buffer());
    std::fs::write(&path, b"corrupt").expect("the fixture file is writable");
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    assert!(
        matches!(engine.load_model(&path, root), Err(ModelLoadError::Parse { path: failed, .. }) if failed == path)
    );

    write_gltf(directory.path(), &gltf_document(), &triangle_buffer());
    assert!(engine.load_model(&path, root).is_ok());
}

#[test]
fn public_api_loads_obj_and_repeated_loads_create_distinct_objects() {
    let directory = directory();
    let path = write_obj(directory.path(), OBJ_TRIANGLE);
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    let first = engine
        .load_model(&path, root)
        .expect("valid OBJ loads through the facade");
    let second = engine
        .load_model(&path, root)
        .expect("reloading creates another scene object");

    assert_ne!(first, second);
}

#[test]
fn public_api_reports_invalid_obj_data() {
    let directory = directory();
    let path = write_obj(directory.path(), "v invalid 0 0\nf 1 2 3\n");
    let mut engine = Engine::new();
    let root = engine.scene().root_node();

    assert!(
        matches!(engine.load_model(&path, root), Err(ModelLoadError::Parse { path: failed, .. }) if failed == path)
    );
}

#[test]
fn public_api_reports_missing_mtl_file() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "mtllib colors.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    );
    let mut engine = Engine::new();
    let root = engine.scene().root_node();
    let expected = directory.path().join(MTL_FILE);

    assert!(
        matches!(engine.load_model(&path, root), Err(ModelLoadError::Io { path, .. }) if path == expected)
    );
}

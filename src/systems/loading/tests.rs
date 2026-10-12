use serde_json::json;

use crate::math::Vec3;
use crate::scene::api::SceneApi;
use crate::scene::node::SceneNode;
use crate::scene::viewpoint::ViewpointState;

use super::*;

#[path = "../../../tests/support/mod.rs"]
mod support;

use support::*;

#[test]
fn gltf_reads_positions_and_indices_from_external_buffer() {
    let directory = directory();
    let path = write_gltf(directory.path(), &gltf_document(), &triangle_buffer());

    let mesh = read_mesh(&path).expect("the triangle fixture is valid glTF");

    assert_eq!(mesh.submeshes.len(), 1);
    assert_eq!(mesh.submeshes[0].indices, [0, 1, 2]);
    assert_eq!(mesh.submeshes[0].vertices[1].position, Vec3::X);
    assert_eq!(mesh.color, DEFAULT_MODEL_COLOR);
}

#[test]
fn gltf_reads_embedded_base64_buffer() {
    let directory = directory();
    let mut document = gltf_document();
    embed_buffer(&mut document, &triangle_buffer());
    let path = write_gltf(directory.path(), &document, &[]);

    let mesh = read_mesh(&path).expect("embedded buffers supply all geometry data");

    assert_eq!(mesh.submeshes[0].indices, [0, 1, 2]);
}

#[test]
fn glb_reads_binary_chunk() {
    let directory = directory();
    let path = write_glb(directory.path(), gltf_document(), &triangle_buffer());

    let mesh = read_mesh(&path).expect("the GLB fixture has valid JSON and binary chunks");

    assert_eq!(mesh.submeshes[0].vertices[2].position, Vec3::Y);
}

#[test]
fn gltf_generates_indices_for_nonindexed_geometry() {
    let directory = directory();
    let mut document = gltf_document();
    document["meshes"][0]["primitives"][0]
        .as_object_mut()
        .expect("primitive is an object")
        .remove("indices");
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    let mesh = read_mesh(&path).expect("a triangle can omit its index accessor");

    assert_eq!(mesh.submeshes[0].indices, [0, 1, 2]);
}

#[test]
fn gltf_composes_parent_and_child_transforms_for_mesh_instances() {
    let directory = directory();
    let mut document = gltf_document();
    document["nodes"] = json!([
        {"translation": [10, 0, 0], "rotation": [0, 0, 1, 0], "children": [1, 2]},
        {"mesh": 0, "translation": [2, 0, 0], "scale": [2, 3, 1]},
        {"mesh": 0, "translation": [0, 5, 0]}
    ]);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    let mesh = read_mesh(&path).expect("mesh instances preserve their node hierarchy");

    assert_eq!(mesh.submeshes.len(), 2);
    assert_eq!(
        mesh.submeshes[0].vertices[0].position,
        Vec3::new(8.0, 0.0, 0.0)
    );
    assert_eq!(
        mesh.submeshes[0].vertices[1].position,
        Vec3::new(6.0, 0.0, 0.0)
    );
    assert_eq!(
        mesh.submeshes[1].vertices[0].position,
        Vec3::new(10.0, -5.0, 0.0)
    );
}

#[test]
fn gltf_reverses_triangle_winding_for_mirrored_nodes() {
    let directory = directory();
    let mut document = gltf_document();
    document["nodes"][0]["scale"] = json!([-1, 1, 1]);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    let mesh = read_mesh(&path).expect("a negative scale preserves visible triangle faces");

    assert_eq!(mesh.submeshes[0].indices, [0, 2, 1]);
}

#[test]
fn gltf_selects_first_explicit_color_on_used_geometry() {
    let directory = directory();
    let mut document = gltf_document();
    document["materials"] = json!([
        {"pbrMetallicRoughness": {"baseColorFactor": [0, 0, 1, 1]}},
        {},
        {"pbrMetallicRoughness": {"baseColorFactor": [0.2, 0.4, 0.6, 0.8]}},
        {"pbrMetallicRoughness": {"baseColorFactor": [1, 0, 0, 1]}}
    ]);
    let primitive = document["meshes"][0]["primitives"][0].clone();
    document["meshes"][0]["primitives"] = json!([primitive.clone(), primitive.clone(), primitive]);
    for (index, material) in [1, 2, 3].into_iter().enumerate() {
        document["meshes"][0]["primitives"][index]["material"] = json!(material);
    }
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    let mesh = read_mesh(&path).expect("all three primitives and materials are valid");

    assert_eq!(mesh.color, [0.2, 0.4, 0.6, 0.8]);
}

#[test]
fn gltf_material_without_explicit_color_uses_neutral_color() {
    let directory = directory();
    let mut document = gltf_document();
    document["materials"] = json!([{}]);
    document["meshes"][0]["primitives"][0]["material"] = json!(0);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert_eq!(
        read_mesh(&path).expect("an empty material is valid").color,
        DEFAULT_MODEL_COLOR
    );
}

#[test]
fn gltf_missing_external_buffer_reports_its_path_and_io_cause() {
    let directory = directory();
    let path = write_gltf(directory.path(), &gltf_document(), &triangle_buffer());
    let buffer_path = directory.path().join(BUFFER_FILE);
    std::fs::remove_file(&buffer_path).expect("the generated buffer exists");

    assert!(
        matches!(read_mesh(&path), Err(ModelLoadError::Io { path, source })
        if path == buffer_path && source.kind() == std::io::ErrorKind::NotFound)
    );
}

#[test]
fn gltf_rejects_out_of_range_indices() {
    let directory = directory();
    let mut buffer = triangle_buffer();
    buffer[40..42].copy_from_slice(&99_u16.to_le_bytes());
    let path = write_gltf(directory.path(), &gltf_document(), &buffer);

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_rejects_truncated_buffer() {
    let directory = directory();
    let path = write_gltf(directory.path(), &gltf_document(), &triangle_buffer()[..36]);

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_rejects_accessor_outside_buffer_view() {
    let directory = directory();
    let mut document = gltf_document();
    document["accessors"][0]["count"] = json!(4);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_rejects_non_triangle_geometry() {
    let directory = directory();
    let mut document = gltf_document();
    document["meshes"][0]["primitives"][0]["mode"] = json!(1);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_rejects_cyclic_node_hierarchy() {
    let directory = directory();
    let mut document = gltf_document();
    document["nodes"][0]["children"] = json!([0]);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_ignores_missing_texture_images() {
    let directory = directory();
    let mut document = gltf_document();
    document["images"] = json!([{"uri": "missing.png"}]);
    document["textures"] = json!([{"source": 0}]);
    document["materials"] = json!([{"pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}}]);
    document["meshes"][0]["primitives"][0]["material"] = json!(0);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert!(read_mesh(&path).is_ok());
}

#[test]
fn gltf_ignores_buffers_used_only_by_images() {
    let directory = directory();
    let mut document = gltf_document();
    document["buffers"]
        .as_array_mut()
        .expect("buffers is an array")
        .push(json!({"byteLength": 8, "uri": "missing-image.bin"}));
    document["bufferViews"]
        .as_array_mut()
        .expect("bufferViews is an array")
        .push(json!({"buffer": 1, "byteLength": 8}));
    document["images"] = json!([{"bufferView": 2, "mimeType": "image/png"}]);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert!(read_mesh(&path).is_ok());
}

#[test]
fn gltf_rejects_nonfinite_positions() {
    let directory = directory();
    let mut buffer = triangle_buffer();
    buffer[..4].copy_from_slice(&f32::NAN.to_le_bytes());
    let path = write_gltf(directory.path(), &gltf_document(), &buffer);

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_rejects_malformed_embedded_base64() {
    let directory = directory();
    let mut document = gltf_document();
    document["buffers"][0]["uri"] = json!("data:application/octet-stream;base64,!");
    let path = write_gltf(directory.path(), &document, &[]);

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn gltf_rejects_sparse_accessor_with_clear_reason() {
    let directory = directory();
    let mut document = gltf_document();
    document["accessors"][0]["sparse"] = json!({
        "count": 1,
        "indices": {"bufferView": 1, "componentType": 5123},
        "values": {"bufferView": 0}
    });
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    assert!(
        matches!(read_mesh(&path), Err(ModelLoadError::Parse { message, .. }) if message.contains("sparse"))
    );
}

#[test]
fn gltf_reads_interleaved_position_accessor() {
    let directory = directory();
    let mut document = gltf_document();
    let mut buffer = Vec::new();
    for position in [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        buffer.extend(
            position
                .into_iter()
                .chain([123.0])
                .flat_map(f32::to_le_bytes),
        );
    }
    buffer.extend([0_u16, 1, 2].into_iter().flat_map(u16::to_le_bytes));
    document["buffers"][0]["byteLength"] = json!(54);
    document["bufferViews"][0]["byteLength"] = json!(48);
    document["bufferViews"][0]["byteStride"] = json!(16);
    document["bufferViews"][1]["byteOffset"] = json!(48);
    let path = write_gltf(directory.path(), &document, &buffer);

    let mesh = read_mesh(&path).expect("POSITION can be interleaved with unused attributes");

    assert_eq!(mesh.submeshes[0].vertices[1].position, Vec3::X);
    assert_eq!(mesh.submeshes[0].vertices[2].position, Vec3::Y);
}

#[test]
fn gltf_uses_selected_scene_without_loading_other_scenes() {
    let directory = directory();
    let mut document = gltf_document();
    document["nodes"] = json!([{"mesh": 0}, {"mesh": 0, "translation": [7, 0, 0]}]);
    document["scenes"] = json!([{"nodes": [0]}, {"nodes": [1]}]);
    document["scene"] = json!(1);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());

    let mesh = read_mesh(&path).expect("the default scene selects only its own instances");

    assert_eq!(mesh.submeshes.len(), 1);
    assert_eq!(
        mesh.submeshes[0].vertices[0].position,
        Vec3::new(7.0, 0.0, 0.0)
    );
}

#[test]
fn failed_load_preserves_existing_scene_and_resources() {
    let directory = directory();
    let path = write_gltf(directory.path(), &gltf_document(), &triangle_buffer());
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let root = scene.root_node();
    let first =
        load_model(&path, &mut resources, &mut scene, root).expect("the first triangle is valid");
    let original = resources.meshes().to_vec();
    std::fs::write(&path, b"corrupt").expect("the fixture file is writable");

    assert!(load_model(&path, &mut resources, &mut scene, root).is_err());

    assert_eq!(resources.meshes(), original);
    assert_eq!(scene.objects().len(), 1);
    assert_eq!(scene.nodes()[0].objects, [first]);
}

#[test]
fn unknown_parent_is_rejected_before_file_io_or_resource_registration() {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();

    let result = load_model(
        Path::new("missing.gltf"),
        &mut resources,
        &mut scene,
        SceneNodeId(99),
    );

    assert!(matches!(
        result,
        Err(ModelLoadError::Scene(SceneError::UnknownNode(SceneNodeId(
            99
        ))))
    ));
    assert!(resources.meshes().is_empty());
    assert!(scene.objects().is_empty());
}

#[test]
fn composite_gltf_creates_one_object_attached_to_requested_node() {
    let directory = directory();
    let mut document = gltf_document();
    document["nodes"] =
        json!([{"mesh": 0, "children": [1]}, {"mesh": 0, "translation": [2, 0, 0]}]);
    let path = write_gltf(directory.path(), &document, &triangle_buffer());
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let root = scene.root_node();
    scene.restore(
        vec![SceneNode::new(None), SceneNode::new(Some(root))],
        vec![],
        ViewpointState::default(),
    );
    let parent = SceneNodeId(1);

    let object = load_model(&path, &mut resources, &mut scene, parent)
        .expect("the model and parent are valid");

    assert_eq!(scene.objects().len(), 1);
    assert!(scene.nodes()[0].objects.is_empty());
    assert_eq!(scene.nodes()[1].objects, [object]);
    assert_eq!(resources.meshes()[0].submeshes.len(), 2);
}

#[test]
fn obj_loads_triangle_without_material_library() {
    let directory = directory();
    let path = write_obj(directory.path(), OBJ_TRIANGLE);

    let mesh = read_mesh(&path).expect("positions and a triangle form a valid OBJ model");

    assert_eq!(mesh.submeshes.len(), 1);
    assert_eq!(mesh.submeshes[0].indices, [0, 1, 2]);
    assert_eq!(mesh.submeshes[0].vertices[1].position, Vec3::X);
    assert_eq!(mesh.color, DEFAULT_MODEL_COLOR);
}

#[test]
fn obj_triangulates_quad_with_negative_indices() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf -4 -3 -2 -1\n",
    );

    let mesh =
        read_mesh(&path).expect("negative OBJ indices are relative to the current vertex list");

    assert_eq!(mesh.submeshes[0].indices, [0, 1, 2, 0, 2, 3]);
    assert_eq!(mesh.submeshes[0].vertices.len(), 4);
}

#[test]
fn obj_reads_separate_position_uv_and_normal_indices() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/3/1 2/2/1 3/1/1\n",
    );

    let mesh = read_mesh(&path).expect("UV and normal indices do not replace position indices");

    assert_eq!(mesh.submeshes[0].vertices[1].position, Vec3::X);
    assert_eq!(mesh.submeshes[0].vertices[2].position, Vec3::Y);
}

#[test]
fn obj_preserves_object_parts_as_one_mesh() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 3 0 0\nv 4 0 0\nv 3 1 0\no first\nf 1 2 3\ng second\nf 4 5 6\n",
    );

    let mesh = read_mesh(&path).expect("objects and groups partition one model into submeshes");

    assert_eq!(mesh.submeshes.len(), 2);
    assert_eq!(
        mesh.submeshes[1].vertices[0].position,
        Vec3::new(3.0, 0.0, 0.0)
    );
}

#[test]
fn obj_selects_first_diffuse_color_used_by_geometry() {
    let directory = directory();
    std::fs::write(directory.path().join(MTL_FILE), "newmtl unused\nKd 0 0 1\nnewmtl empty\nnewmtl green\nKd 0.2 0.4 0.6\nd 0.8\nnewmtl red\nKd 1 0 0\n").expect("the fixture directory is writable");
    let path = write_obj(
        directory.path(),
        "mtllib colors.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl empty\nf 1 2 3\nusemtl green\nf 1 2 3\nusemtl red\nf 1 2 3\n",
    );

    let mesh = read_mesh(&path).expect("valid MTL colors follow the common selection rule");

    assert_eq!(mesh.submeshes.len(), 3);
    assert_eq!(mesh.color, [0.2, 0.4, 0.6, 0.8]);
}

#[test]
fn obj_material_without_diffuse_color_uses_neutral_color() {
    let directory = directory();
    std::fs::write(
        directory.path().join(MTL_FILE),
        "newmtl empty\nKa 0.1 0.2 0.3\n",
    )
    .expect("the fixture directory is writable");
    let path = write_obj(
        directory.path(),
        "mtllib colors.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl empty\nf 1 2 3\n",
    );

    assert_eq!(
        read_mesh(&path)
            .expect("ambient color does not define the diffuse color")
            .color,
        DEFAULT_MODEL_COLOR
    );
}

#[test]
fn obj_missing_material_library_reports_its_path_and_io_cause() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "mtllib colors.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    );
    let expected = directory.path().join(MTL_FILE);

    assert!(
        matches!(read_mesh(&path), Err(ModelLoadError::Io { path, source })
        if path == expected && source.kind() == std::io::ErrorKind::NotFound)
    );
}

#[test]
fn obj_corrupt_material_library_reports_the_material_file() {
    let directory = directory();
    std::fs::write(
        directory.path().join(MTL_FILE),
        "newmtl broken\nKd red green blue\n",
    )
    .expect("the fixture directory is writable");
    let path = write_obj(
        directory.path(),
        "mtllib colors.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    );
    let expected = directory.path().join(MTL_FILE);

    assert!(
        matches!(read_mesh(&path), Err(ModelLoadError::Parse { path, .. }) if path == expected)
    );
}

#[test]
fn obj_does_not_hide_material_library_errors_after_a_successful_library() {
    let directory = directory();
    std::fs::write(directory.path().join(MTL_FILE), "newmtl green\nKd 0 1 0\n")
        .expect("the fixture directory is writable");
    let path = write_obj(
        directory.path(),
        "mtllib colors.mtl\nmtllib missing.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl green\nf 1 2 3\n",
    );

    assert!(matches!(read_mesh(&path), Err(ModelLoadError::Io { .. })));
}

#[test]
fn obj_rejects_out_of_range_indices_without_partial_scene_changes() {
    let directory = directory();
    let path = write_obj(directory.path(), "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 99\n");
    let mut resources = ResourceManager::new();
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    assert!(matches!(
        load_model(&path, &mut resources, &mut scene, root),
        Err(ModelLoadError::Parse { .. })
    ));

    assert!(resources.meshes().is_empty());
    assert!(scene.objects().is_empty());
}

#[test]
fn obj_rejects_invalid_vertex_numbers() {
    let directory = directory();
    let path = write_obj(directory.path(), "v nope 0 0\nf 1 2 3\n");

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn obj_rejects_nonfinite_positions() {
    let directory = directory();
    let path = write_obj(directory.path(), "v NaN 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n");

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn obj_rejects_models_without_faces() {
    let directory = directory();
    let path = write_obj(directory.path(), "v 0 0 0\n");

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn obj_rejects_concave_polygon() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 2 0 0\nv 1 0.5 0\nv 2 2 0\nv 0 2 0\nf 1 2 3 4 5\n",
    );

    assert!(
        matches!(read_mesh(&path), Err(ModelLoadError::Parse { message, .. }) if message.contains("convex"))
    );
}

#[test]
fn obj_rejects_nonplanar_quad() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 1\nf 1 2 3 4\n",
    );

    assert!(
        matches!(read_mesh(&path), Err(ModelLoadError::Parse { message, .. }) if message.contains("planar"))
    );
}

#[test]
fn obj_rejects_line_geometry() {
    let directory = directory();
    let path = write_obj(directory.path(), "v 0 0 0\nv 1 0 0\nl 1 2\n");

    assert!(matches!(
        read_mesh(&path),
        Err(ModelLoadError::Parse { .. })
    ));
}

#[test]
fn obj_triangulates_convex_polygon_with_more_than_four_vertices() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 2 0 0\nv 3 1 0\nv 1.5 2 0\nv 0 1 0\nf 1 2 3 4 5\n",
    );

    let mesh = read_mesh(&path).expect("a convex planar pentagon supports fan triangulation");

    assert_eq!(mesh.submeshes[0].indices, [0, 1, 2, 0, 2, 3, 0, 3, 4]);
}

#[test]
fn composite_obj_creates_one_scene_object() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\no first\nf 1 2 3\no second\nf 1 2 3\n",
    );
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let root = scene.root_node();

    let object = load_model(&path, &mut resources, &mut scene, root)
        .expect("both parts belong to the same model");

    assert_eq!(scene.nodes()[0].objects, [object]);
    assert_eq!(scene.objects().len(), 1);
    assert_eq!(resources.meshes().len(), 1);
    assert_eq!(resources.meshes()[0].submeshes.len(), 2);
}

#[test]
fn obj_invalid_later_part_does_not_register_the_valid_first_part() {
    let directory = directory();
    let path = write_obj(
        directory.path(),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\no first\nf 1 2 3\no broken\nf 1 2 99\n",
    );
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let root = scene.root_node();

    assert!(load_model(&path, &mut resources, &mut scene, root).is_err());

    assert!(scene.objects().is_empty());
    assert!(resources.meshes().is_empty());
}

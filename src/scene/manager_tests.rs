//! Unit tests for [`SceneManager`], including its engine-internal operations.

use super::*;
use crate::math::Vec3;
use crate::resources::library::{BuiltinMesh, MeshLibrary};
use crate::resources::manager::ResourceManager;
use crate::scene::api::SceneApi;

const EPSILON: f32 = 1e-5;
const WINDOW: WindowSize = WindowSize {
    width: 800,
    height: 600,
};

fn add_cube(resources: &mut ResourceManager) -> MeshHandle {
    resources.add_mesh(MeshLibrary::build(BuiltinMesh::Cube))
}

fn world_origin(item: &RenderItem) -> Vec3 {
    item.world_transform.transform_point3(Vec3::ZERO)
}

#[test]
fn new_scene_has_a_root_without_parent() {
    let scene = SceneManager::new();
    let root = scene.root_node();

    assert_eq!(scene.nodes()[root.0 as usize].parent, None);
    assert!(scene.nodes()[root.0 as usize].children.is_empty());
}

#[test]
fn default_scene_uses_the_default_viewpoint() {
    let scene = SceneManager::new();

    assert_eq!(scene.viewpoint(), ViewpointState::default());
}

#[test]
fn frame_camera_is_placed_at_the_viewpoint() {
    let mut scene = SceneManager::new();
    let position = Vec3::new(1.0, 2.0, 3.0);
    let viewpoint = ViewpointState {
        position,
        direction: Vec3::new(0.0, 0.0, -1.0),
        fov_y: 60.0_f32.to_radians(),
    };
    scene
        .set_viewpoint(viewpoint)
        .expect("the scene accepts a valid viewpoint");

    let frame = scene.build_render_frame(WINDOW);

    let origin = frame.camera.view.transform_point3(position);
    assert!(origin.abs_diff_eq(Vec3::ZERO, EPSILON));
}

#[test]
fn zero_height_window_produces_finite_projection() {
    let scene = SceneManager::new();

    let frame = scene.build_render_frame(WindowSize::new(800, 0));

    assert!(frame.camera.projection.is_finite());
}

#[test]
fn create_object_attaches_to_the_root() {
    let mut resources = ResourceManager::new();
    let mesh = resources.builtin(BuiltinMesh::Cube);
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    let id = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    assert_eq!(scene.objects().len(), 1);
    assert_eq!(scene.nodes()[0].objects, vec![id]);
}

#[test]
fn create_object_with_unknown_parent_fails() {
    let mut resources = ResourceManager::new();
    let mesh = resources.builtin(BuiltinMesh::Cube);
    let mut scene = SceneManager::new();

    let result = scene.create_object(mesh, SceneNodeId(99));

    assert!(matches!(result, Err(SceneError::UnknownNode(_))));
}

#[test]
fn create_node_attaches_to_its_parent() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    let child = scene.create_node(root).expect("the root node exists");

    assert_eq!(scene.nodes()[child.0 as usize].parent, Some(root));
    assert_eq!(scene.nodes()[root.0 as usize].children, vec![child]);
}

#[test]
fn create_node_with_unknown_parent_fails() {
    let mut scene = SceneManager::new();

    let result = scene.create_node(SceneNodeId(99));

    assert!(matches!(result, Err(SceneError::UnknownNode(_))));
}

#[test]
fn set_node_transform_updates_the_local_transform() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));

    scene
        .set_node_transform(root, transform)
        .expect("the root node exists");

    assert_eq!(scene.nodes()[root.0 as usize].local_transform, transform);
}

#[test]
fn set_node_transform_with_unknown_node_fails() {
    let mut scene = SceneManager::new();

    let result = scene.set_node_transform(SceneNodeId(99), Transform::identity());

    assert!(matches!(result, Err(SceneError::UnknownNode(_))));
}

#[test]
fn reparent_object_moves_it_between_nodes() {
    let mut resources = ResourceManager::new();
    let mesh = resources.builtin(BuiltinMesh::Cube);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let child = scene.create_node(root).expect("the root node exists");
    let object = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    scene
        .reparent_object(object, child)
        .expect("the object and the new parent exist");

    assert!(scene.nodes()[root.0 as usize].objects.is_empty());
    assert_eq!(scene.nodes()[child.0 as usize].objects, vec![object]);
}

#[test]
fn reparent_unknown_object_fails() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    let result = scene.reparent_object(SceneObjectId(99), root);

    assert!(matches!(result, Err(SceneError::UnknownObject(_))));
}

#[test]
fn reparent_object_to_unknown_node_fails() {
    let mut resources = ResourceManager::new();
    let mesh = resources.builtin(BuiltinMesh::Cube);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let object = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    let result = scene.reparent_object(object, SceneNodeId(99));

    assert!(matches!(result, Err(SceneError::UnknownNode(_))));
    assert_eq!(scene.nodes()[root.0 as usize].objects, vec![object]);
}

#[test]
fn frame_collects_objects_with_accumulated_world_transforms() {
    let mut resources = ResourceManager::new();
    let mesh = resources.builtin(BuiltinMesh::Cube);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let child = scene.create_node(root).expect("the root node exists");
    scene
        .set_node_transform(
            child,
            Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
        )
        .expect("the child node exists");
    scene
        .create_object(mesh, child)
        .expect("the child node exists");

    let frame = scene.build_render_frame(WINDOW);

    assert_eq!(frame.items.len(), 1);
    assert_eq!(frame.items[0].mesh, mesh);
    let origin = frame.items[0].world_transform.transform_point3(Vec3::ZERO);
    assert!(origin.abs_diff_eq(Vec3::new(0.0, 0.0, -5.0), EPSILON));
}

#[test]
fn frame_has_no_items_for_an_empty_scene() {
    let scene = SceneManager::new();

    let frame = scene.build_render_frame(WINDOW);

    assert!(frame.items.is_empty());
}

#[test]
fn create_node_ids_are_monotonic_and_distinct() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    let first = scene.create_node(root).expect("the root node exists");
    let second = scene.create_node(root).expect("the root node exists");

    assert_ne!(first, second);
    assert!(second.0 > first.0);
    assert_eq!(scene.nodes()[root.0 as usize].children, vec![first, second]);
}

#[test]
fn create_object_ids_are_monotonic_and_distinct() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    let first = scene
        .create_object(mesh, root)
        .expect("the root node exists");
    let second = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    assert_ne!(first, second);
    assert!(second.0 > first.0);
    assert_eq!(scene.nodes()[root.0 as usize].objects, vec![first, second]);
}

#[test]
fn set_node_transform_rejects_a_translation_kept_without_change() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let child = scene.create_node(root).expect("the root node exists");
    let transform = Transform::from_translation(Vec3::new(4.0, 0.0, 0.0));

    scene
        .set_node_transform(child, transform)
        .expect("the child node exists");

    assert_eq!(scene.nodes()[child.0 as usize].local_transform, transform);
    assert_eq!(
        scene.nodes()[root.0 as usize].local_transform,
        Transform::identity()
    );
}

#[test]
fn set_viewpoint_replaces_the_state() {
    let mut scene = SceneManager::new();
    let viewpoint = ViewpointState {
        position: Vec3::new(9.0, 8.0, 7.0),
        direction: Vec3::new(1.0, 0.0, 0.0),
        fov_y: 30.0_f32.to_radians(),
    };

    scene
        .set_viewpoint(viewpoint)
        .expect("the scene accepts a valid viewpoint");

    assert_eq!(scene.viewpoint(), viewpoint);
}

#[test]
fn frame_items_keep_the_creation_order_of_objects() {
    let mut resources = ResourceManager::new();
    let first_mesh = add_cube(&mut resources);
    let second_mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    scene
        .create_object(first_mesh, root)
        .expect("the root node exists");
    scene
        .create_object(second_mesh, root)
        .expect("the root node exists");

    let frame = scene.build_render_frame(WINDOW);

    let meshes: Vec<MeshHandle> = frame.items.iter().map(|item| item.mesh).collect();
    assert_eq!(meshes, vec![first_mesh, second_mesh]);
}

#[test]
fn frame_composes_transforms_across_multiple_levels() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let parent = scene.create_node(root).expect("the root node exists");
    let child = scene.create_node(parent).expect("the parent node exists");
    scene
        .set_node_transform(
            parent,
            Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
        )
        .expect("the parent node exists");
    scene
        .set_node_transform(child, Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)))
        .expect("the child node exists");
    scene
        .create_object(mesh, child)
        .expect("the child node exists");

    let frame = scene.build_render_frame(WINDOW);

    assert_eq!(frame.items.len(), 1);
    assert!(world_origin(&frame.items[0]).abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), EPSILON));
}

#[test]
fn frame_accumulates_parent_child_scale_and_translation() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let parent = scene.create_node(root).expect("the root node exists");
    let child = scene.create_node(parent).expect("the parent node exists");
    scene
        .set_node_transform(
            parent,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        )
        .expect("the parent node exists");
    scene
        .set_node_transform(
            child,
            Transform {
                translation: Vec3::new(1.0, 0.0, 0.0),
                rotation: glam::Quat::IDENTITY,
                scale: Vec3::splat(2.0),
            },
        )
        .expect("the child node exists");
    scene
        .create_object(mesh, child)
        .expect("the child node exists");

    let frame = scene.build_render_frame(WINDOW);

    let point = frame.items[0]
        .world_transform
        .transform_point3(Vec3::new(1.0, 0.0, 0.0));
    assert!(point.abs_diff_eq(Vec3::new(13.0, 0.0, 0.0), EPSILON));
}

#[test]
fn frame_reflects_a_reparent_after_the_change() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let child = scene.create_node(root).expect("the root node exists");
    scene
        .set_node_transform(child, Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)))
        .expect("the child node exists");
    let object = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    let before = scene.build_render_frame(WINDOW);
    assert!(world_origin(&before.items[0]).abs_diff_eq(Vec3::ZERO, EPSILON));

    scene
        .reparent_object(object, child)
        .expect("the object and the new parent exist");

    let after = scene.build_render_frame(WINDOW);
    assert!(world_origin(&after.items[0]).abs_diff_eq(Vec3::new(0.0, 5.0, 0.0), EPSILON));
}

#[test]
fn reparenting_to_the_same_node_keeps_a_single_attachment() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let object = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    scene
        .reparent_object(object, root)
        .expect("the object and the new parent exist");

    assert_eq!(scene.nodes()[root.0 as usize].objects, vec![object]);
    assert_eq!(scene.build_render_frame(WINDOW).items.len(), 1);
}

#[test]
fn reparent_object_leaves_unknown_object_untouched() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let object = scene
        .create_object(mesh, root)
        .expect("the root node exists");

    let result = scene.reparent_object(SceneObjectId(99), root);

    assert!(matches!(result, Err(SceneError::UnknownObject(_))));
    assert_eq!(scene.nodes()[root.0 as usize].objects, vec![object]);
}

#[test]
fn frame_collects_objects_from_every_level_of_the_tree() {
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let child = scene.create_node(root).expect("the root node exists");
    let grandchild = scene.create_node(child).expect("the child node exists");
    scene
        .create_object(mesh, root)
        .expect("the root node exists");
    scene
        .create_object(mesh, child)
        .expect("the child node exists");
    scene
        .create_object(mesh, grandchild)
        .expect("the grandchild node exists");

    let frame = scene.build_render_frame(WINDOW);

    assert_eq!(frame.items.len(), 3);
}

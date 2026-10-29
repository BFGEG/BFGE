//! Unit tests for [`SceneManager`], including its engine-internal operations.

#![cfg(test)]

use super::*;
use crate::math::Vec3;
use crate::resources::library::{BuiltinMesh, MeshLibrary};
use crate::resources::manager::ResourceManager;
use crate::scene::api::SceneApi;
use crate::scene::lighting::{DirectionalLight, SceneLighting};

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
fn clone_object_shares_geometry_and_inherits_the_target_transform() {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let root = scene.root_node();
    let child = scene.create_node(root).unwrap();
    scene
        .set_node_transform(child, Transform::from_translation(Vec3::X))
        .unwrap();
    let source = scene.create_object(mesh, root).unwrap();

    let instance = scene.clone_object(source, child).unwrap();
    let frame = scene.build_render_frame(WINDOW);

    assert_ne!(source, instance);
    assert_eq!(resources.meshes().len(), 1);
    assert_eq!(frame.items.len(), 2);
    assert_eq!(frame.items[0].mesh, frame.items[1].mesh);
    assert_eq!(world_origin(&frame.items[1]), Vec3::X);
}

#[test]
fn clone_object_rejects_unknown_source_without_changes() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();

    assert!(matches!(
        scene.clone_object(SceneObjectId(99), root),
        Err(SceneError::UnknownObject(SceneObjectId(99)))
    ));
    assert!(scene.objects().is_empty());
    assert!(scene.nodes()[0].objects.is_empty());
}

#[test]
fn clone_object_rejects_unknown_parent_without_changes() {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let root = scene.root_node();
    let source = scene.create_object(mesh, root).unwrap();
    let before = scene.build_render_frame(WINDOW);

    assert!(matches!(
        scene.clone_object(source, SceneNodeId(99)),
        Err(SceneError::UnknownNode(SceneNodeId(99)))
    ));
    assert_eq!(scene.build_render_frame(WINDOW).items, before.items);
    assert_eq!(scene.objects().len(), 1);
}

#[test]
fn remove_object_preserves_other_instances_and_the_hierarchy() {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let root = scene.root_node();
    let child = scene.create_node(root).unwrap();
    let source = scene.create_object(mesh, root).unwrap();
    let instance = scene.clone_object(source, child).unwrap();

    scene.remove_object(source).unwrap();
    let frame = scene.build_render_frame(WINDOW);

    assert_eq!(frame.items.len(), 1);
    assert_eq!(frame.items[0].mesh, mesh);
    assert_eq!(scene.nodes().len(), 2);
    assert_eq!(scene.nodes()[0].children, [child]);
    assert!(scene.nodes()[0].objects.is_empty());
    assert_eq!(scene.nodes()[child.0 as usize].objects, [instance]);
    assert!(resources.mesh(mesh).is_some());
}

#[test]
fn removed_identifiers_stay_invalid_after_creating_another_object() {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let mesh = add_cube(&mut resources);
    let root = scene.root_node();
    let removed = scene.create_object(mesh, root).unwrap();
    let survivor = scene.create_object(mesh, root).unwrap();
    scene.remove_object(removed).unwrap();

    let new_object = scene.clone_object(survivor, root).unwrap();

    assert_ne!(removed, new_object);
    assert_ne!(survivor, new_object);
    assert!(matches!(
        scene.remove_object(removed),
        Err(SceneError::UnknownObject(id)) if id == removed
    ));
    assert!(matches!(
        scene.clone_object(removed, root),
        Err(SceneError::UnknownObject(id)) if id == removed
    ));
    assert!(matches!(
        scene.reparent_object(removed, root),
        Err(SceneError::UnknownObject(id)) if id == removed
    ));
    assert_eq!(scene.build_render_frame(WINDOW).items.len(), 2);
}

#[test]
fn remove_object_rejects_unknown_identifier_without_changes() {
    let mut scene = SceneManager::new();

    assert!(matches!(
        scene.remove_object(SceneObjectId(99)),
        Err(SceneError::UnknownObject(SceneObjectId(99)))
    ));
    assert!(scene.objects().is_empty());
    assert!(scene.nodes()[0].objects.is_empty());
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
fn lighting_change_appears_in_the_next_render_frame() {
    let mut scene = SceneManager::new();
    let lighting = SceneLighting {
        ambient: Vec3::splat(0.1),
        directional: DirectionalLight {
            direction: Vec3::X,
            color: Vec3::new(1.0, 0.5, 0.25),
        },
    };

    scene.set_lighting(lighting).expect("the lighting is valid");

    assert_eq!(scene.lighting(), lighting);
    assert_eq!(scene.build_render_frame(WINDOW).lighting, lighting);
}

#[test]
fn invalid_lighting_keeps_the_previous_state() {
    let mut scene = SceneManager::new();
    let original = scene.lighting();
    let invalid = SceneLighting {
        directional: DirectionalLight {
            direction: Vec3::ZERO,
            ..original.directional
        },
        ..original
    };

    assert!(matches!(
        scene.set_lighting(invalid),
        Err(SceneError::InvalidLightDirection)
    ));
    assert_eq!(scene.lighting(), original);
}

#[test]
fn negative_light_color_is_rejected() {
    let mut scene = SceneManager::new();
    let invalid = SceneLighting {
        ambient: Vec3::new(-0.1, 0.0, 0.0),
        ..SceneLighting::default()
    };

    assert!(matches!(
        scene.set_lighting(invalid),
        Err(SceneError::InvalidLightColor)
    ));
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

#[test]
fn node_transform_returns_a_copy_of_the_local_transform() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let child = scene.create_node(root).unwrap();
    scene
        .set_node_transform(root, Transform::from_translation(Vec3::X))
        .unwrap();
    let local = Transform::from_scale(Vec3::splat(2.0));
    scene.set_node_transform(child, local).unwrap();

    let mut copy = scene.node_transform(child).unwrap();
    copy.translation = Vec3::Y;

    assert_eq!(scene.node_transform(child).unwrap(), local);
    assert_ne!(copy, local);
    assert!(matches!(
        scene.node_transform(SceneNodeId(99)),
        Err(SceneError::UnknownNode(_))
    ));
}

#[test]
fn hidden_object_can_be_shown_without_hiding_siblings_or_children() {
    let mut scene = SceneManager::new();
    let mesh = MeshHandle::new(0);
    let root = scene.root_node();
    let child = scene.create_node(root).unwrap();
    let object = scene.create_object(mesh, root).unwrap();
    scene.create_object(mesh, root).unwrap();
    scene.create_object(mesh, child).unwrap();
    assert_eq!(scene.build_render_frame(WINDOW).items.len(), 3);

    scene.set_object_visible(object, false).unwrap();
    assert_eq!(scene.build_render_frame(WINDOW).items.len(), 2);
    scene.set_object_visible(object, true).unwrap();
    assert_eq!(scene.build_render_frame(WINDOW).items.len(), 3);
}

#[test]
fn color_override_is_independent_of_shared_geometry_and_can_be_reset() {
    let mut scene = SceneManager::new();
    let mesh = MeshHandle::new(0);
    let root = scene.root_node();
    let source = scene.create_object(mesh, root).unwrap();
    let instance = scene.clone_object(source, root).unwrap();
    let color = [0.1, 0.3, 1.0];

    scene.set_object_color(instance, color).unwrap();
    let frame = scene.build_render_frame(WINDOW);
    assert_eq!(frame.items[0].color_override, None);
    assert_eq!(frame.items[1].color_override, Some(color));
    assert_eq!(frame.items[0].mesh, frame.items[1].mesh);

    scene.reset_object_color(instance).unwrap();
    assert_eq!(
        scene.build_render_frame(WINDOW).items[1].color_override,
        None
    );
}

#[test]
fn invalid_object_colors_leave_the_previous_override_intact() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let object = scene.create_object(MeshHandle::new(0), root).unwrap();
    let valid = [0.2, 0.4, 0.6];
    scene.set_object_color(object, valid).unwrap();

    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1] {
        for component in 0..3 {
            let mut color = valid;
            color[component] = invalid;
            assert!(matches!(
                scene.set_object_color(object, color),
                Err(SceneError::InvalidObjectColor)
            ));
            assert_eq!(
                scene.build_render_frame(WINDOW).items[0].color_override,
                Some(valid)
            );
        }
    }
}

#[test]
fn property_setters_reject_unknown_and_removed_objects() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let removed = scene.create_object(MeshHandle::new(0), root).unwrap();
    scene.remove_object(removed).unwrap();

    for object in [removed, SceneObjectId(99)] {
        assert!(
            matches!(scene.set_object_visible(object, false), Err(SceneError::UnknownObject(id)) if id == object)
        );
        assert!(
            matches!(scene.set_object_color(object, [1.0; 3]), Err(SceneError::UnknownObject(id)) if id == object)
        );
        assert!(
            matches!(scene.reset_object_color(object), Err(SceneError::UnknownObject(id)) if id == object)
        );
    }
    assert!(scene.build_render_frame(WINDOW).items.is_empty());
}

#[test]
fn clone_copies_properties_and_subsequent_changes_are_independent() {
    let mut scene = SceneManager::new();
    let root = scene.root_node();
    let source = scene.create_object(MeshHandle::new(0), root).unwrap();
    let color = [0.1, 0.8, 0.3];
    scene.set_object_color(source, color).unwrap();
    scene.set_object_visible(source, false).unwrap();

    let instance = scene.clone_object(source, root).unwrap();
    assert!(scene.build_render_frame(WINDOW).items.is_empty());
    scene.set_object_visible(instance, true).unwrap();
    assert_eq!(
        scene.build_render_frame(WINDOW).items[0].color_override,
        Some(color)
    );
    scene.reset_object_color(instance).unwrap();
    scene.set_object_visible(source, true).unwrap();
    let frame = scene.build_render_frame(WINDOW);
    assert_eq!(frame.items[0].color_override, Some(color));
    assert_eq!(frame.items[1].color_override, None);
}

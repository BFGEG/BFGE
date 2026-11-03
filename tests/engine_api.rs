use BFGE::math::Vec3;
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineError, SceneError, SceneLighting, ViewpointState,
    WindowDescriptor, WindowSize,
};

const TEST_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");

#[test]
fn public_scene_api_can_instance_and_remove_loaded_objects() {
    let mut engine = Engine::new();
    let root = engine.scene().root_node();
    let object = engine
        .load_model(std::path::Path::new(TEST_MODEL), root)
        .unwrap();
    let instance = engine.scene().clone_object(object, root).unwrap();

    engine.scene().remove_object(object).unwrap();

    assert!(
        matches!(engine.scene().clone_object(object, root), Err(SceneError::UnknownObject(id)) if id == object)
    );
    assert!(
        matches!(engine.scene().reparent_object(object, root), Err(SceneError::UnknownObject(id)) if id == object)
    );
    let next = engine.scene().clone_object(instance, root).unwrap();
    assert_ne!(next, object);
    assert_ne!(next, instance);
}

#[test]
fn new_engine_exposes_default_viewpoint() {
    let mut engine = Engine::new();

    assert_eq!(engine.scene().viewpoint(), ViewpointState::default());
}

#[test]
fn scene_changes_are_retained_through_the_engine_api() {
    let mut engine = Engine::default();
    let viewpoint = ViewpointState {
        fov_y: 45.0_f32.to_radians(),
        ..ViewpointState::default()
    };

    engine
        .scene()
        .set_viewpoint(viewpoint)
        .expect("the scene accepts a valid perspective viewpoint");

    assert_eq!(engine.scene().viewpoint(), viewpoint);
}

#[test]
fn engine_scene_exposes_mutable_lighting() {
    let mut engine = Engine::new();
    let lighting = SceneLighting {
        ambient: Vec3::splat(0.2),
        directional: DirectionalLight {
            direction: Vec3::NEG_Y,
            color: Vec3::new(0.8, 0.7, 0.6),
        },
    };

    engine
        .scene()
        .set_lighting(lighting)
        .expect("the lighting is valid");

    assert_eq!(engine.scene().lighting(), lighting);
}

#[test]
fn run_requires_initialization() {
    let mut engine = Engine::new();

    assert!(matches!(engine.run(), Err(EngineError::NotInitialized)));
}

#[test]
fn shutdown_requires_initialization() {
    let mut engine = Engine::new();

    assert!(matches!(
        engine.shutdown(),
        Err(EngineError::NotInitialized)
    ));
}

#[test]
fn resize_requires_initialization() {
    let mut engine = Engine::new();

    assert!(matches!(
        engine.set_window_size(WindowSize::new(800, 600)),
        Err(EngineError::NotInitialized)
    ));
}

#[test]
fn invalid_window_size_leaves_engine_uninitialized() {
    let mut engine = Engine::new();

    assert!(matches!(
        engine.initialize(WindowDescriptor::new("Engine API test", 0, 600)),
        Err(EngineError::Platform(_))
    ));
    assert!(matches!(engine.run(), Err(EngineError::NotInitialized)));
}

#[test]
fn public_api_preserves_object_properties_through_save_and_restore() {
    let mut engine = Engine::new();
    let root = engine.scene().root_node();
    let node = engine.scene().create_node(root).unwrap();
    let object = engine
        .load_model(std::path::Path::new(TEST_MODEL), node)
        .unwrap();
    let mut transform = engine.scene().node_transform(node).unwrap();
    transform.translation = Vec3::X;
    engine.scene().set_node_transform(node, transform).unwrap();
    engine
        .scene()
        .set_object_color(object, [0.2, 0.4, 0.6])
        .unwrap();
    engine.scene().set_object_visible(object, false).unwrap();
    engine.save_scene().unwrap();
    engine.scene().remove_object(object).unwrap();

    engine.restore_scene().unwrap();

    assert_eq!(engine.scene().node_transform(node).unwrap(), transform);
    engine.scene().set_object_visible(object, true).unwrap();
    engine.scene().reset_object_color(object).unwrap();
}

use BFGE::{Engine, EngineApi, EngineError, ViewpointState, WindowDescriptor, WindowSize};

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
fn unavailable_platform_leaves_engine_uninitialized() {
    let mut engine = Engine::new();

    assert!(matches!(
        engine.initialize(WindowDescriptor::new("Engine API test", 800, 600)),
        Err(EngineError::Platform(_))
    ));
    assert!(matches!(engine.run(), Err(EngineError::NotInitialized)));
}

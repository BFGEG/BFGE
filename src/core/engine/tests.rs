use std::cell::RefCell;
use std::rc::Rc;

use crate::math::Vec3;
use crate::platform::Window;
use crate::render::frame::RenderFrame;
use crate::render::renderer::Renderer;
use crate::render::stats::RenderStats;
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::library::{BuiltinMesh, MeshLibrary};
use crate::resources::manager::ResourceManager;
use crate::scene::transform::Transform;

use super::*;

const BACKEND_FAILURE: &str = "test backend failure";
const TEST_WINDOW_TITLE: &str = "Engine lifecycle test";
const TEST_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");
const UNSUPPORTED_EXTENSION: &str = "unsupported";
const TEST_SIZE: WindowSize = WindowSize {
    width: 800,
    height: 600,
};

#[derive(Default)]
struct BackendState {
    initialized: bool,
    fail_initialize: bool,
    fail_resize: bool,
    fail_render: bool,
    calls: Vec<Call>,
    frames: Vec<RenderFrame>,
    size: Option<WindowSize>,
    shutdown_count: usize,
}

#[derive(Debug, PartialEq)]
enum Call {
    Started(usize),
    Render,
    Ended(usize),
    Resize(WindowSize),
}

struct TestRenderSystem(Rc<RefCell<BackendState>>);

impl RenderSystem for TestRenderSystem {
    fn initialize(&mut self, _window: &Window) -> Result<(), RenderError> {
        let mut state = self.0.borrow_mut();
        if state.fail_initialize {
            Err(RenderError::Initialization(BACKEND_FAILURE.to_owned()))
        } else {
            state.initialized = true;
            Ok(())
        }
    }

    fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError> {
        let mut state = self.0.borrow_mut();
        state.calls.push(Call::Resize(size));
        if state.fail_resize {
            Err(RenderError::Render(BACKEND_FAILURE.to_owned()))
        } else {
            state.size = Some(size);
            Ok(())
        }
    }

    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderStats, RenderError> {
        let mut state = self.0.borrow_mut();
        state.calls.push(Call::Render);
        if state.fail_render {
            Err(RenderError::Render(BACKEND_FAILURE.to_owned()))
        } else {
            assert!(
                frame
                    .items
                    .iter()
                    .all(|item| resources.mesh(item.mesh).is_some())
            );
            state.frames.push(frame.clone());
            Ok(RenderStats::default())
        }
    }

    fn shutdown(&mut self) {
        self.0.borrow_mut().shutdown_count += 1;
    }
}

fn engine_with_backend() -> (Engine, Rc<RefCell<BackendState>>) {
    let state = Rc::new(RefCell::new(BackendState::default()));
    let mut engine = Engine::new();
    engine.parts.renderer = Renderer::new(Box::new(TestRenderSystem(Rc::clone(&state))));
    (engine, state)
}

#[test]
fn initialization_delegates_to_backend_before_committing_platform() {
    let (mut engine, state) = engine_with_backend();

    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    assert!(state.borrow().initialized);
    assert!(engine.platform.is_some());
}

#[test]
fn backend_initialization_failure_preserves_uninitialized_state_and_cause() {
    let (mut engine, state) = engine_with_backend();
    state.borrow_mut().fail_initialize = true;

    let result = engine.initialize_platform(PlatformAdapter::for_test(TEST_SIZE));

    assert!(matches!(result,
        Err(EngineError::Lifecycle(lifecycle::LifecycleError::Render(
            RenderError::Initialization(message)
        ))) if message == BACKEND_FAILURE
    ));
    assert!(engine.platform.is_none());

    state.borrow_mut().fail_initialize = false;
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("a failed initialization leaves the engine available for retry");
}

#[test]
fn initialized_engine_rejects_reinitialization() {
    let (mut engine, _) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    assert!(matches!(
        engine.initialize(WindowDescriptor::new(TEST_WINDOW_TITLE, 800, 600)),
        Err(EngineError::AlreadyInitialized)
    ));
}

#[test]
fn resize_delegates_requested_size_to_backend() {
    let (mut engine, state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");
    let size = WindowSize::new(1280, 720);

    engine
        .set_window_size(size)
        .expect("the test platform and backend accept resizing");

    assert_eq!(state.borrow().size, Some(size));
}

#[test]
fn resize_preserves_backend_error_cause() {
    let (mut engine, state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");
    state.borrow_mut().fail_resize = true;

    assert!(matches!(engine.set_window_size(WindowSize::new(1280, 720)),
        Err(EngineError::Render(RenderError::Render(message))) if message == BACKEND_FAILURE
    ));
    assert_eq!(
        engine
            .platform
            .as_ref()
            .map(|platform| platform.window().size()),
        Some(TEST_SIZE)
    );
}

#[test]
fn invalid_resize_does_not_change_backend_or_window() {
    let (mut engine, state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    assert!(matches!(
        engine.set_window_size(WindowSize::new(0, 600)),
        Err(EngineError::Platform(
            crate::platform::PlatformError::InvalidWindowSize(_)
        ))
    ));
    assert_eq!(state.borrow().size, None);
    assert_eq!(
        engine
            .platform
            .as_ref()
            .map(|platform| platform.window().size()),
        Some(TEST_SIZE)
    );
}

#[test]
fn shutdown_releases_platform_and_calls_backend_once() {
    let (mut engine, state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    engine
        .shutdown()
        .expect("the test backend supports shutdown");

    assert!(engine.platform.is_none());
    assert!(matches!(
        engine.shutdown(),
        Err(EngineError::NotInitialized)
    ));
    assert_eq!(state.borrow().shutdown_count, 1);
}

#[test]
fn event_registration_retains_handlers_without_calling_them() {
    let mut engine = Engine::new();
    let calls = Rc::new(RefCell::new(Vec::new()));

    for index in 0..2 {
        let started = Rc::clone(&calls);
        let ended = Rc::clone(&calls);
        engine
            .add_event(EngineEvent {
                frame_started: Box::new(move |_, _| {
                    started.borrow_mut().push((index, true));
                    Ok(())
                }),
                frame_ended: Box::new(move |_, _| {
                    ended.borrow_mut().push((index, false));
                    Ok(())
                }),
            })
            .expect("frame handlers can be registered before initialization");
    }

    assert!(calls.borrow().is_empty());
    let time = crate::core::engine_loop::EventTime {
        time_since_last_event: 0.0,
        time_since_last_frame: 0.0,
    };
    let mut context = FrameContext::new(&mut engine.parts.scene, &mut engine.parts.resources);
    for event in engine.event_loop.callbacks() {
        (event.frame_started)(&mut context, &time).expect("the registered test handler succeeds");
        (event.frame_ended)(&mut context, &time).expect("the registered test handler succeeds");
    }
    assert_eq!(
        *calls.borrow(),
        [(0, true), (0, false), (1, true), (1, false)]
    );
}

#[test]
fn invalid_window_size_fails_before_event_loop_acquisition() {
    let mut engine = Engine::new();

    assert!(matches!(
        engine.initialize(WindowDescriptor::new(TEST_WINDOW_TITLE, 0, 600)),
        Err(EngineError::Platform(
            crate::platform::PlatformError::InvalidWindowSize(_)
        ))
    ));
    assert!(engine.winit_loop.is_none());
}

#[test]
fn scene_survives_save_shutdown_reinitialize_and_restore() {
    let (mut engine, _state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    let root = engine.parts.scene.root_node();
    let child = engine
        .parts
        .scene
        .create_node(root)
        .expect("the root node exists");
    engine
        .parts
        .scene
        .set_node_transform(
            child,
            Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
        )
        .expect("the created node exists");
    let mesh = engine
        .parts
        .resources
        .add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
    let object = engine
        .parts
        .scene
        .create_object(mesh, child)
        .expect("the created node exists");

    let before = engine.parts.scene.build_render_frame(TEST_SIZE);
    engine.save_scene().expect("in-memory save cannot fail");
    engine
        .shutdown()
        .expect("the test backend supports shutdown");

    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("reinitialization succeeds for the test backend");
    assert!(engine.parts.scene.objects().is_empty());

    engine
        .restore_scene()
        .expect("restore succeeds into an empty scene");
    let after = engine.parts.scene.build_render_frame(TEST_SIZE);
    assert_eq!(before.camera, after.camera);
    assert_eq!(before.items, after.items);
    assert!(
        engine
            .parts
            .scene
            .objects()
            .get(object.0 as usize)
            .is_some()
    );
    let expected = MeshLibrary::build(BuiltinMesh::Cube);
    assert_eq!(engine.parts.resources.mesh(mesh), Some(&expected));
}

#[test]
fn restore_rejects_non_empty_scene_without_prior_save() {
    let (mut engine, _state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    let root = engine.parts.scene.root_node();
    let mesh = engine
        .parts
        .resources
        .add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
    engine
        .parts
        .scene
        .create_object(mesh, root)
        .expect("the root node exists");

    assert!(matches!(
        engine.restore_scene(),
        Err(ScenePersistenceError::SceneNotEmpty)
    ));
}

#[test]
fn restore_without_save_into_empty_scene_is_a_noop() {
    let (mut engine, _state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");

    assert!(engine.restore_scene().is_ok());
    assert!(engine.parts.scene.objects().is_empty());
    assert!(engine.parts.scene.nodes().len() == 1);
}

#[test]
fn store_and_frame_handlers_survive_shutdown_and_reinitialization() {
    let (mut engine, _state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");
    engine
        .parts
        .resources
        .add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
    engine.save_scene().expect("in-memory save cannot fail");
    let registered = engine.event_loop.callbacks().len();
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(|_, _| Ok(())),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .expect("frame handlers can be registered before initialization");

    engine
        .shutdown()
        .expect("the test backend supports shutdown");
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("reinitialization succeeds for the test backend");

    assert_eq!(engine.event_loop.callbacks().len(), registered + 1);
    engine
        .restore_scene()
        .expect("restore succeeds into an empty scene");
    assert_eq!(engine.parts.resources.meshes().len(), 1);
}

fn initialized_engine() -> (Engine, Rc<RefCell<BackendState>>) {
    let (mut engine, state) = engine_with_backend();
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the test backend accepts initialization");
    (engine, state)
}

fn register_recorders(engine: &mut Engine, state: &Rc<RefCell<BackendState>>) {
    for index in 0..2 {
        let started = Rc::clone(state);
        let ended = Rc::clone(state);
        engine
            .add_event(EngineEvent {
                frame_started: Box::new(move |_, _| {
                    started.borrow_mut().calls.push(Call::Started(index));
                    Ok(())
                }),
                frame_ended: Box::new(move |_, _| {
                    ended.borrow_mut().calls.push(Call::Ended(index));
                    Ok(())
                }),
            })
            .expect("frame handler registration succeeds");
    }
}

fn run_batches(
    engine: &mut Engine,
    batches: impl IntoIterator<Item = Vec<PlatformEvent>>,
) -> Result<(), EngineError> {
    let mut batches = batches.into_iter();
    let time = Instant::now();
    engine.run_with_events(
        |_, _| {
            Ok(batches
                .next()
                .expect("the loop must stop within the scripted events"))
        },
        || time,
    )
}

#[test]
fn frame_handlers_surround_render_in_registration_order() {
    let (mut engine, state) = initialized_engine();
    register_recorders(&mut engine, &state);

    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]])
        .expect("one frame succeeds before closing");

    assert_eq!(
        state.borrow().calls,
        [
            Call::Started(0),
            Call::Started(1),
            Call::Render,
            Call::Ended(0),
            Call::Ended(1),
        ]
    );
}

#[test]
fn frame_started_scene_changes_reach_the_current_frame() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    let mesh = engine.parts.resources.builtin(BuiltinMesh::Cube);
    engine
        .parts
        .scene
        .create_object(mesh, root)
        .expect("the root exists");
    let translation = Vec3::new(1.0, 2.0, -3.0);
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                let scene = context.scene();
                scene
                    .set_node_transform(root, Transform::from_translation(translation))
                    .map_err(Into::into)
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .expect("frame handler registration succeeds");

    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]])
        .expect("one frame succeeds before closing");

    let state = state.borrow();
    assert_eq!(state.frames.len(), 1);
    assert_eq!(state.frames[0].items.len(), 1);
    assert_eq!(state.frames[0].items[0].mesh, mesh);
    assert_eq!(
        state.frames[0].items[0].world_transform,
        crate::math::Mat4::from_translation(translation)
    );
}

#[test]
fn frame_started_can_load_and_place_a_model_in_the_current_frame() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    let child = engine.scene().create_node(root).unwrap();
    let translation = Vec3::new(1.0, 2.0, -3.0);
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                context.load_model(Path::new(TEST_MODEL), child)?;
                context
                    .scene()
                    .set_node_transform(child, Transform::from_translation(translation))?;
                Ok(())
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .unwrap();

    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]]).unwrap();

    let state = state.borrow();
    assert_eq!(state.frames[0].items.len(), 1);
    assert_eq!(
        state.frames[0].items[0].world_transform,
        crate::math::Mat4::from_translation(translation)
    );
    assert_eq!(engine.parts.scene.nodes().len(), 2);
    assert_eq!(engine.parts.resources.meshes().len(), 1);
}

#[test]
fn frame_ended_model_loading_is_visible_in_the_next_frame() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    let mut loaded = false;
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(|_, _| Ok(())),
            frame_ended: Box::new(move |context, _| {
                if !loaded {
                    context.load_model(Path::new(TEST_MODEL), root)?;
                    loaded = true;
                }
                Ok(())
            }),
        })
        .unwrap();

    run_batches(
        &mut engine,
        [vec![], vec![], vec![PlatformEvent::CloseRequested]],
    )
    .unwrap();

    let state = state.borrow();
    assert!(state.frames[0].items.is_empty());
    assert_eq!(state.frames[1].items.len(), 1);
}

#[test]
fn frame_started_removal_excludes_the_object_from_the_current_frame() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    let object = engine.load_model(Path::new(TEST_MODEL), root).unwrap();
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                context.scene().remove_object(object)?;
                Ok(())
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .unwrap();

    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]]).unwrap();

    assert!(state.borrow().frames[0].items.is_empty());
    assert!(engine.scene().remove_object(object).is_err());
}

#[test]
fn frame_ended_removal_is_visible_in_the_next_frame() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    let mut object = Some(engine.load_model(Path::new(TEST_MODEL), root).unwrap());
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(|_, _| Ok(())),
            frame_ended: Box::new(move |context, _| {
                if let Some(object) = object.take() {
                    context.scene().remove_object(object)?;
                }
                Ok(())
            }),
        })
        .unwrap();

    run_batches(
        &mut engine,
        [vec![], vec![], vec![PlatformEvent::CloseRequested]],
    )
    .unwrap();

    let state = state.borrow();
    assert_eq!(state.frames[0].items.len(), 1);
    assert!(state.frames[1].items.is_empty());
}

#[test]
fn runtime_load_failure_preserves_source_and_state_and_stops_the_loop() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                context.load_model(
                    Path::new(TEST_MODEL)
                        .with_extension(UNSUPPORTED_EXTENSION)
                        .as_path(),
                    root,
                )?;
                Ok(())
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .unwrap();
    register_recorders(&mut engine, &state);

    let result = run_batches(&mut engine, [vec![]]);

    assert!(matches!(
        result,
        Err(EngineError::ModelLoad(ModelLoadError::UnsupportedFormat(_)))
    ));
    assert!(state.borrow().calls.is_empty());
    assert!(engine.parts.scene.objects().is_empty());
    assert!(engine.parts.resources.meshes().is_empty());
    assert!(!engine.event_loop.is_running());
    assert_eq!(engine.event_loop.callbacks().len(), 3);
}

#[test]
fn frame_ended_scene_changes_reach_the_next_frame() {
    let (mut engine, state) = initialized_engine();
    let initial_camera = engine.parts.scene.build_render_frame(TEST_SIZE).camera;
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(|_, _| Ok(())),
            frame_ended: Box::new(|context, _| {
                let scene = context.scene();
                let mut viewpoint = scene.viewpoint();
                viewpoint.position += Vec3::X;
                scene.set_viewpoint(viewpoint).map_err(Into::into)
            }),
        })
        .expect("frame handler registration succeeds");

    run_batches(
        &mut engine,
        [vec![], vec![], vec![PlatformEvent::CloseRequested]],
    )
    .expect("two frames succeed before closing");

    let state = state.borrow();
    assert_eq!(state.frames.len(), 2);
    assert_eq!(state.frames[0].camera, initial_camera);
    assert_ne!(state.frames[1].camera.view, initial_camera.view);
}

#[test]
fn frame_started_error_preserves_cause_and_skips_remaining_steps() {
    let (mut engine, state) = initialized_engine();
    let root = engine.scene().root_node();
    let translation = Vec3::X;
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                let scene = context.scene();
                scene.set_node_transform(root, Transform::from_translation(translation))?;
                Err(crate::SceneError::UnknownNode(SceneNodeId(42)).into())
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .expect("frame handler registration succeeds");
    register_recorders(&mut engine, &state);

    let result = run_batches(&mut engine, [vec![]]);

    assert!(matches!(
        result,
        Err(EngineError::Scene(crate::SceneError::UnknownNode(
            SceneNodeId(42)
        )))
    ));
    assert!(state.borrow().calls.is_empty());
    assert!(!engine.event_loop.is_running());
    assert!(engine.event_loop.next_internal().is_none());
    assert_eq!(
        engine.parts.scene.nodes()[root.0 as usize]
            .local_transform
            .translation,
        translation
    );
}

#[test]
fn render_error_preserves_cause_and_skips_frame_ended() {
    let (mut engine, state) = initialized_engine();
    register_recorders(&mut engine, &state);
    state.borrow_mut().fail_render = true;

    let result = run_batches(&mut engine, [vec![]]);

    assert!(
        matches!(result, Err(EngineError::Render(RenderError::Render(message))) if message == BACKEND_FAILURE)
    );
    assert_eq!(
        state.borrow().calls,
        [Call::Started(0), Call::Started(1), Call::Render]
    );
    assert!(!engine.event_loop.is_running());
    assert!(engine.event_loop.next_internal().is_none());
}

#[test]
fn frame_ended_error_preserves_cause_and_skips_later_handlers() {
    let (mut engine, state) = initialized_engine();
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(|_, _| Ok(())),
            frame_ended: Box::new(|_, _| Err(crate::SceneError::InvalidReparent.into())),
        })
        .expect("frame handler registration succeeds");
    register_recorders(&mut engine, &state);

    let result = run_batches(&mut engine, [vec![]]);

    assert!(matches!(
        result,
        Err(EngineError::Scene(crate::SceneError::InvalidReparent))
    ));
    assert_eq!(
        state.borrow().calls,
        [Call::Started(0), Call::Started(1), Call::Render]
    );
    assert!(!engine.event_loop.is_running());
}

#[test]
fn close_before_first_frame_skips_handlers_and_render() {
    let (mut engine, state) = initialized_engine();
    register_recorders(&mut engine, &state);

    run_batches(
        &mut engine,
        [vec![
            PlatformEvent::CloseRequested,
            PlatformEvent::Resized(TEST_SIZE),
        ]],
    )
    .expect("closing succeeds");

    assert!(state.borrow().calls.is_empty());
    assert!(!engine.event_loop.is_running());
    assert!(engine.event_loop.next_internal().is_none());
    assert!(engine.platform.is_some());
    assert_eq!(state.borrow().shutdown_count, 0);
}

#[test]
fn resize_reconfigures_backend_before_building_the_frame() {
    let (mut engine, state) = initialized_engine();
    let size = WindowSize::new(1280, 720);
    let expected = engine.parts.scene.build_render_frame(size).camera;

    run_batches(
        &mut engine,
        [
            vec![PlatformEvent::Resized(size)],
            vec![PlatformEvent::CloseRequested],
        ],
    )
    .expect("resize and rendering succeed");

    let state = state.borrow();
    assert_eq!(state.calls, [Call::Resize(size), Call::Render]);
    assert_eq!(state.frames[0].camera, expected);
}

#[test]
fn resize_error_clears_pending_events_and_allows_retry() {
    let (mut engine, state) = initialized_engine();
    state.borrow_mut().fail_resize = true;

    let result = run_batches(
        &mut engine,
        [vec![
            PlatformEvent::Resized(TEST_SIZE),
            PlatformEvent::CloseRequested,
        ]],
    );

    assert!(
        matches!(result, Err(EngineError::Render(RenderError::Render(message))) if message == BACKEND_FAILURE)
    );
    assert!(!engine.event_loop.is_running());
    assert!(engine.event_loop.next_internal().is_none());
    state.borrow_mut().fail_resize = false;
    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]])
        .expect("retry must not consume the previous run's close or tick");
    assert_eq!(state.borrow().frames.len(), 1);
}

#[test]
fn minimized_window_skips_frames_until_restored() {
    for minimized in [WindowSize::new(0, 600), WindowSize::new(800, 0)] {
        let (mut engine, state) = initialized_engine();
        register_recorders(&mut engine, &state);

        run_batches(
            &mut engine,
            [
                vec![PlatformEvent::Resized(minimized)],
                vec![],
                vec![PlatformEvent::Resized(TEST_SIZE)],
                vec![PlatformEvent::CloseRequested],
            ],
        )
        .expect("minimizing and restoring succeed");

        assert_eq!(
            state.borrow().calls,
            [
                Call::Resize(minimized),
                Call::Resize(TEST_SIZE),
                Call::Started(0),
                Call::Started(1),
                Call::Render,
                Call::Ended(0),
                Call::Ended(1),
            ]
        );
    }
}

#[test]
fn pump_error_stops_the_loop_and_preserves_cause() {
    let (mut engine, state) = initialized_engine();

    let result = engine.run_with_events(
        |_, _| Err(EngineError::Platform(PlatformError::MissingEventLoop)),
        Instant::now,
    );

    assert!(matches!(
        result,
        Err(EngineError::Platform(PlatformError::MissingEventLoop))
    ));
    assert!(!engine.event_loop.is_running());
    assert!(engine.event_loop.next_internal().is_none());
    assert!(state.borrow().calls.is_empty());
}

#[test]
fn frame_budget_accounts_for_render_work() {
    let (mut engine, _) = initialized_engine();
    let initial = Instant::now();
    let mut current = initial;
    let mut timeouts = Vec::new();
    engine
        .run_with_events(
            |_, timeout| {
                timeouts.push(timeout);
                Ok(if timeouts.len() == 1 {
                    vec![]
                } else {
                    vec![PlatformEvent::CloseRequested]
                })
            },
            || {
                let result = current;
                current += Duration::from_millis(1);
                result
            },
        )
        .expect("the loop closes on the second pump");

    assert_eq!(
        timeouts,
        [Duration::ZERO, FRAME_INTERVAL - Duration::from_millis(2)]
    );
}

#[test]
fn handlers_receive_distinct_event_deltas_and_shared_frame_delta() {
    let (mut engine, _) = initialized_engine();
    let times = Rc::new(RefCell::new(Vec::new()));
    let started = Rc::clone(&times);
    let ended = Rc::clone(&times);
    engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |_, time| {
                started.borrow_mut().push(*time);
                Ok(())
            }),
            frame_ended: Box::new(move |_, time| {
                ended.borrow_mut().push(*time);
                Ok(())
            }),
        })
        .expect("frame handler registration succeeds");
    let mut batches = [vec![], vec![], vec![PlatformEvent::CloseRequested]].into_iter();
    let initial = Instant::now();
    let mut current = initial;

    engine
        .run_with_events(
            |_, _| Ok(batches.next().expect("the loop closes on the third pump")),
            || {
                let result = current;
                current += Duration::from_millis(1);
                result
            },
        )
        .expect("two frames succeed");

    let times = times.borrow();
    assert_eq!(times.len(), 4);
    assert_eq!(times[0].time_since_last_frame, 0.002);
    assert_eq!(times[0].time_since_last_event, 0.003);
    assert_eq!(
        times[1].time_since_last_frame,
        times[0].time_since_last_frame
    );
    assert_eq!(times[1].time_since_last_event, 0.001);
    assert_eq!(times[2].time_since_last_frame, 0.005);
    assert_eq!(
        times[3].time_since_last_frame,
        times[2].time_since_last_frame
    );
}

#[test]
fn run_shutdown_and_reinitialize_preserve_handlers() {
    let (mut engine, state) = initialized_engine();
    register_recorders(&mut engine, &state);
    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]])
        .expect("the first run succeeds");
    engine
        .shutdown()
        .expect("the initialized engine shuts down");
    assert!(matches!(engine.run(), Err(EngineError::NotInitialized)));
    engine
        .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
        .expect("the backend can be initialized again");
    run_batches(&mut engine, [vec![], vec![PlatformEvent::CloseRequested]])
        .expect("the second run succeeds");

    assert_eq!(state.borrow().frames.len(), 2);
    assert_eq!(
        state
            .borrow()
            .calls
            .iter()
            .filter(|call| **call == Call::Started(0))
            .count(),
        2
    );
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[test]
#[ignore = "requires a desktop display and a presentation-capable GPU; opens a window for three frames"]
fn native_engine_loads_model_and_drives_frames() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    use winit::platform::windows::EventLoopBuilderExtWindows;
    #[cfg(target_os = "linux")]
    use winit::platform::x11::EventLoopBuilderExtX11;

    let mut event_loop = EventLoop::builder().with_any_thread(true).build()?;
    let platform = PlatformAdapter::new(
        WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
        &mut event_loop,
    )?;
    let mut engine = Engine::new();
    engine.winit_loop = Some(event_loop);
    engine.initialize_platform(platform)?;
    let root = engine.scene().root_node();
    let node = engine.scene().create_node(root)?;
    let mut object = None;
    let mut frame = 0;
    let completed = Rc::new(RefCell::new(0));
    let ended = Rc::clone(&completed);
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, _| {
            frame += 1;
            if frame == 1 {
                object = Some(context.load_model(Path::new(TEST_MODEL), node)?);
            } else if let Some(previous) = object.take() {
                if frame == 2 {
                    object = Some(context.scene().clone_object(previous, node)?);
                }
                context.scene().remove_object(previous)?;
            }
            let scene = context.scene();
            scene
                .set_node_transform(node, Transform::from_euler(Vec3::new(0.2, 0.4, 0.0)))
                .map_err(Into::into)
        }),
        frame_ended: Box::new(move |_, _| {
            *ended.borrow_mut() += 1;
            if *ended.borrow() == 3 {
                Err(crate::SceneError::InvalidReparent.into())
            } else {
                Ok(())
            }
        }),
    })?;

    let result = engine.run();
    engine.shutdown()?;

    assert!(matches!(
        result,
        Err(EngineError::Scene(crate::SceneError::InvalidReparent))
    ));
    assert_eq!(*completed.borrow(), 3);
    assert!(!engine.event_loop.is_running());
    assert!(engine.platform.is_none());
    Ok(())
}

use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineEvent, SceneError, SceneLighting, SceneNodeId,
    Transform, ViewpointState, WindowDescriptor,
};

const WINDOW_TITLE: &str = "BFGE spiral sculpture";
const STEP_MODELS: [&str; 4] = [
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/cube_coral.obj"
    ),
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_teal.obj"),
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_gold.obj"),
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/cube_violet.obj"
    ),
];
const BASE_MODEL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/cube_slate.obj"
);
const STEPS: usize = 96;
const TOUR_SECONDS: f32 = 24.0;

fn add_node(
    engine: &mut Engine,
    parent: SceneNodeId,
    transform: Transform,
) -> Result<SceneNodeId, SceneError> {
    let node = engine.scene().create_node(parent)?;
    engine.scene().set_node_transform(node, transform)?;
    Ok(node)
}

fn add_box(
    engine: &mut Engine,
    model: &Path,
    parent: SceneNodeId,
    transform: Transform,
) -> Result<(), Box<dyn Error>> {
    let node = add_node(engine, parent, transform)?;
    engine.load_model(model, node)?;
    Ok(())
}

fn build_spiral(engine: &mut Engine) -> Result<SceneNodeId, Box<dyn Error>> {
    let root = engine.scene().root_node();
    let pivot = add_node(engine, root, Transform::identity())?;
    add_box(
        engine,
        Path::new(BASE_MODEL),
        root,
        Transform {
            translation: Vec3::new(0.0, -6.1, 0.0),
            scale: Vec3::new(16.0, 0.2, 16.0),
            ..Transform::identity()
        },
    )?;
    add_box(
        engine,
        Path::new(BASE_MODEL),
        pivot,
        Transform {
            translation: Vec3::new(0.0, 1.5, 0.0),
            scale: Vec3::new(0.8, 14.0, 0.8),
            ..Transform::identity()
        },
    )?;
    for step in 0..STEPS {
        let angle = step as f32 * 0.45;
        add_box(
            engine,
            Path::new(STEP_MODELS[step % STEP_MODELS.len()]),
            pivot,
            Transform {
                translation: Vec3::new(
                    5.0 * angle.cos(),
                    -5.0 + step as f32 * 0.14,
                    5.0 * angle.sin(),
                ),
                rotation: Quat::from_rotation_y(-angle),
                scale: Vec3::new(1.0, 0.35, 1.6),
            },
        )?;
    }
    Ok(pivot)
}

fn camera_at(elapsed: f32) -> ViewpointState {
    let angle = elapsed.rem_euclid(TOUR_SECONDS) * TAU / TOUR_SECONDS;
    let position = Vec3::new(
        14.0 * angle.sin(),
        2.0 - 6.0 * angle.cos(),
        14.0 * angle.cos(),
    );
    ViewpointState {
        position,
        direction: (Vec3::new(0.0, 1.5, 0.0) - position).normalize_or_zero(),
        fov_y: 55.0_f32.to_radians(),
    }
}

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let pivot = build_spiral(engine)?;
    let mut elapsed = 0.0_f32;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |scene, time| {
            elapsed = (elapsed + time.time_since_last_frame).rem_euclid(TOUR_SECONDS);
            let angle = elapsed * TAU / TOUR_SECONDS;
            scene.set_viewpoint(camera_at(elapsed))?;
            scene.set_lighting(SceneLighting {
                ambient: Vec3::splat(0.18),
                directional: DirectionalLight {
                    direction: Vec3::new(angle.sin(), -1.0, angle.cos()),
                    color: Vec3::new(0.55, 0.75, 1.0),
                },
            })?;
            scene.set_node_transform(
                pivot,
                Transform::from_rotation(Quat::from_rotation_y(angle * 2.0)),
            )
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!("Orbiting a rotating sculpture of {STEPS} steps. Close the window to quit.");
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 960, 720))?;
    let result = run_scene(&mut engine);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{Engine, EngineApi, EngineEvent, Transform, ViewpointState, WindowDescriptor};

const WINDOW_TITLE: &str = "BFGE object properties";
const MODEL_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_gold.obj");
const CYCLE_SECONDS: f32 = 6.0;
const RESET_SECONDS: f32 = 4.0;
const VISIBLE_SECONDS: f32 = 3.0;

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let original_node = engine.scene().create_node(root)?;
    let animated_node = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        original_node,
        Transform::from_translation(Vec3::new(-2.0, 0.0, 0.0)),
    )?;
    engine.scene().set_node_transform(
        animated_node,
        Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
    )?;
    let original = engine.load_model(Path::new(MODEL_PATH), original_node)?;
    let animated = engine.scene().clone_object(original, animated_node)?;
    let position = Vec3::new(0.0, 3.0, 10.0);
    engine.scene().set_viewpoint(ViewpointState {
        position,
        direction: -position,
        ..ViewpointState::default()
    })?;
    let mut elapsed = 0.0;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            elapsed = (elapsed + time.time_since_last_frame).rem_euclid(CYCLE_SECONDS);
            let phase = elapsed * TAU / CYCLE_SECONDS;
            let scene = context.scene();
            let mut transform = scene.node_transform(animated_node)?;
            transform.rotation = Quat::from_rotation_y(phase);
            transform.scale = Vec3::splat(1.0 + 0.2 * phase.sin());
            scene.set_node_transform(animated_node, transform)?;
            if elapsed < RESET_SECONDS {
                scene.set_object_color(
                    animated,
                    [0.5 + 0.5 * phase.sin(), 0.2, 0.5 + 0.5 * phase.cos()],
                )?;
            } else {
                scene.reset_object_color(animated)?;
            }
            scene.set_object_visible(
                animated,
                !(VISIBLE_SECONDS..RESET_SECONDS).contains(&elapsed),
            )?;
            Ok(())
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!(
        "Left: original model. Right: changing color and transform, hidden at 3–4 s, original color at 4–6 s. Repeats every 6 s."
    );
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 1000, 700))?;
    let result = run_scene(&mut engine);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

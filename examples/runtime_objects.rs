use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{Engine, EngineApi, EngineEvent, Transform, ViewpointState, WindowDescriptor};

const WINDOW_TITLE: &str = "BFGE runtime objects";
const MODEL_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_gold.obj");
const SLOT_COUNT: usize = 12;
const CHANGE_SECONDS: f32 = 0.3;
const RADIUS: f32 = 3.0;

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let source = engine.load_model(Path::new(MODEL_PATH), root)?;
    let nodes = (0..SLOT_COUNT)
        .map(|_| engine.scene().create_node(root))
        .collect::<Result<Vec<_>, _>>()?;
    let position = Vec3::new(0.0, 5.0, 9.0);
    engine.scene().set_viewpoint(ViewpointState {
        position,
        direction: -position,
        ..ViewpointState::default()
    })?;
    let mut objects = [None; SLOT_COUNT];
    let mut next_slot = 0;
    let mut elapsed = 0.0_f32;
    let mut since_change = CHANGE_SECONDS;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            elapsed = (elapsed + time.time_since_last_frame).rem_euclid(TAU);
            since_change += time.time_since_last_frame;
            let scene = context.scene();
            if since_change >= CHANGE_SECONDS {
                since_change %= CHANGE_SECONDS;
                if let Some(object) = objects[next_slot].take() {
                    scene.remove_object(object)?;
                } else {
                    objects[next_slot] = Some(scene.clone_object(source, nodes[next_slot])?);
                }
                next_slot = (next_slot + 1) % SLOT_COUNT;
            }
            for (index, node) in nodes.iter().enumerate() {
                let angle = TAU * index as f32 / SLOT_COUNT as f32 + elapsed;
                scene.set_node_transform(
                    *node,
                    Transform {
                        translation: Vec3::new(
                            RADIUS * angle.cos(),
                            (angle * 2.0).sin() * 0.5,
                            RADIUS * angle.sin(),
                        ),
                        rotation: Quat::from_rotation_y(-angle),
                        scale: Vec3::splat(0.45 + 0.1 * angle.sin()),
                    },
                )?;
            }
            Ok(())
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!(
        "Cubes appear, move and disappear under twelve fixed nodes. Close the window to quit."
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

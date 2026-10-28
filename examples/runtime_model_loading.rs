use std::error::Error;
use std::f32::consts::TAU;
use std::path::PathBuf;

use BFGE::math::{Quat, Vec3};
use BFGE::{Engine, EngineApi, EngineEvent, Transform, WindowDescriptor};

const WINDOW_TITLE: &str = "BFGE runtime model loading";
const SWITCH_SECONDS: f32 = 3.0;
const DEFAULT_MODELS: [&str; 3] = [
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/cube_coral.obj"
    ),
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_teal.obj"),
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/cube_violet.obj"
    ),
];

fn run_scene(engine: &mut Engine, paths: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let node = engine.scene().create_node(root)?;
    let mut paths = paths.into_iter();
    let mut current = None;
    let mut since_switch = SWITCH_SECONDS;
    let mut angle = 0.0_f32;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            since_switch += time.time_since_last_frame;
            angle = (angle + time.time_since_last_frame).rem_euclid(TAU);
            if since_switch >= SWITCH_SECONDS {
                since_switch %= SWITCH_SECONDS;
                if let Some(path) = paths.next() {
                    let loaded = context.load_model(&path, node)?;
                    if let Some(previous) = current.replace(loaded) {
                        context.scene().remove_object(previous)?;
                    }
                    println!("Loaded during run(): {}", path.display());
                }
            }
            context.scene().set_node_transform(
                node,
                Transform {
                    translation: Vec3::new(angle.sin(), 0.3 * (angle * 2.0).sin(), 0.0),
                    rotation: Quat::from_rotation_y(angle),
                    scale: Vec3::splat(0.9 + 0.2 * angle.sin()),
                },
            )?;
            Ok(())
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!("Models load in sequence every three seconds. Close the window to quit.");
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let paths = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let paths = if paths.is_empty() {
        DEFAULT_MODELS.into_iter().map(PathBuf::from).collect()
    } else {
        paths
    };
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 800, 600))?;
    let result = run_scene(&mut engine, paths);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

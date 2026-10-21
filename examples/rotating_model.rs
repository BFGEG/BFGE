use std::error::Error;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

use BFGE::math::Quat;
use BFGE::{Engine, EngineApi, EngineEvent, Transform, WindowDescriptor};

const WINDOW_TITLE: &str = "BFGE rotating model";
const DEFAULT_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");

fn run_scene(engine: &mut Engine, path: &Path) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let node = engine.scene().create_node(root)?;
    engine.load_model(path, node)?;
    let mut angle = 0.0;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |scene, time| {
            angle = (angle + time.time_since_last_frame) % TAU;
            scene.set_node_transform(node, Transform::from_rotation(Quat::from_rotation_y(angle)))
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MODEL));
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 800, 600))?;
    let result = run_scene(&mut engine, &path);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

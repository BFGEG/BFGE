use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineEvent, SceneLighting, Transform, ViewpointState,
    WindowDescriptor,
};

const WINDOW_TITLE: &str = "BFGE lighting demo";
const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");
const LIGHT_CYCLE_SECONDS: f32 = 8.0;
const INITIAL_POSITION: Vec3 = Vec3::new(5.0, 2.0, 0.0);
const INITIAL_DIRECTION: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
const MOUSE_SENSITIVITY: f32 = 0.002;

fn add_cube(
    engine: &mut Engine,
    model: &Path,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    let node = engine.scene().create_node(root)?;
    engine.scene().set_node_transform(
        node,
        Transform {
            translation,
            rotation,
            scale,
        },
    )?;
    engine.load_model(model, node)?;
    Ok(())
}

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let model = Path::new(CUBE_MODEL);
    add_cube(
        engine,
        model,
        Vec3::new(0.0, 1.0, 0.0),
        Quat::from_rotation_y(0.0),
        Vec3::ONE,
    )?;
    add_cube(
        engine,
        model,
        Vec3::new(0.0, 0.0, 0.0),
        Quat::from_rotation_y(0.0),
        Vec3::new(10.0, 1.0, 10.0),
    )?;
    engine.scene().set_viewpoint(ViewpointState {
        position: INITIAL_POSITION,
        direction: INITIAL_DIRECTION,
        roll: 0.0,
        fov_y: 90.0_f32.to_radians(),
    })?;

    let initial_direction = INITIAL_DIRECTION.normalize();
    let mut yaw = initial_direction.x.atan2(-initial_direction.z);
    let mut pitch = initial_direction.y.asin();

    let mut angle = 0.0_f32;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            let scene = context.scene();
            angle = (angle + time.time_since_last_frame * TAU / LIGHT_CYCLE_SECONDS) % TAU;
            scene
                .set_lighting(SceneLighting {
                    ambient: Vec3::splat(0.2),
                    directional: DirectionalLight {
                        direction: Vec3::new(angle.cos(), -0.8, angle.sin()),
                        color: Vec3::splat(0.9),
                    },
                })
                .map_err(Into::into)
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, _time| {
            let (dx, dy) = context.input().mouse_delta();
            pitch -= dy as f32 * MOUSE_SENSITIVITY;
            yaw += dx as f32 * MOUSE_SENSITIVITY;
            let (yaw_sin, yaw_cos) = yaw.sin_cos();
            let (pitch_sin, pitch_cos) = pitch.sin_cos();
            let mut viewpoint = context.scene().viewpoint();
            viewpoint.direction = Vec3::new(yaw_sin * pitch_cos, pitch_sin, -yaw_cos * pitch_cos);
            context.scene().set_viewpoint(viewpoint)?;
            Ok(())
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 800, 600))?;
    engine.hide_cursor()?;
    let result = run_scene(&mut engine);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

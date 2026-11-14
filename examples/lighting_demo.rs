use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineEvent, SceneLighting, SceneNodeId, Transform,
    ViewpointState, WindowDescriptor,
};

const WINDOW_TITLE: &str = "BFGE lighting demo";
const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");
const TEXTURED_BUNNY_MODEL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/new_stanford_bunny.obj"
);
const LIGHT_CYCLE_SECONDS: f32 = 8.0;

fn add_model(
    file: &str,
    engine: &mut Engine,
    transform: Transform,
) -> Result<SceneNodeId, Box<dyn Error>> {
    let root = engine.scene().root_node();
    let node = engine.scene().create_node(root)?;
    let model = Path::new(file);
    engine.scene().set_node_transform(node, transform)?;
    engine.load_model(model, node)?;
    Ok(node)
}

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let bunny_transform = Transform {
        translation: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quat::from_rotation_y(0.0),
        scale: 10.0 * Vec3::ONE,
    };
    let cube_transform = Transform {
        translation: Vec3::new(0.0, 0.0, -10.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(10.15, 10.15, 1.0),
    };

    let bunny_node = add_model(TEXTURED_BUNNY_MODEL, engine, bunny_transform)?;
    add_model(CUBE_MODEL, engine, cube_transform)?;

    engine.scene().set_viewpoint(ViewpointState {
        position: Vec3::new(0.0, 2.0, 6.5),
        direction: Vec3::new(0.0, -0.25, -1.0),
        fov_y: 55.0_f32.to_radians(),
    })?;

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
        frame_started: Box::new(move |context, time| {
            let scene = context.scene();
            angle = (angle + time.time_since_last_frame) % TAU;
            let new_transform = Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                rotation: Quat::from_rotation_y(angle),
                scale: 10.0 * Vec3::ONE,
            };
            scene
                .set_node_transform(bunny_node, new_transform)
                .map_err(Into::into)
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 800, 600))?;
    let result = run_scene(&mut engine);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

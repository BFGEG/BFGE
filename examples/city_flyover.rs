use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineEvent, SceneLighting, SceneNodeId, Transform,
    ViewpointState, WindowDescriptor,
};

const WINDOW_TITLE: &str = "BFGE city flyover";
const BUILDING_MODELS: [&str; 4] = [
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
const GROUND_MODEL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/cube_slate.obj"
);
const CITY_SIDE: usize = 12;
const BLOCK_SPACING: f32 = 2.3;
const CITY_OFFSET: f32 = (CITY_SIDE as f32 - 1.0) * BLOCK_SPACING * 0.5;
const TOUR_SECONDS: f32 = 30.0;

fn add_box(
    engine: &mut Engine,
    model: &Path,
    parent: SceneNodeId,
    translation: Vec3,
    scale: Vec3,
) -> Result<(), Box<dyn Error>> {
    let node = engine.scene().create_node(parent)?;
    engine.scene().set_node_transform(
        node,
        Transform {
            translation,
            rotation: Quat::IDENTITY,
            scale,
        },
    )?;
    engine.load_model(model, node)?;
    Ok(())
}

fn build_city(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    add_box(
        engine,
        Path::new(GROUND_MODEL),
        root,
        Vec3::new(0.0, -0.2, 0.0),
        Vec3::new(30.0, 0.3, 30.0),
    )?;
    for row in 0..CITY_SIDE {
        for column in 0..CITY_SIDE {
            let height = 1.0 + ((column * 17 + row * 11) % 8) as f32 * 0.75;
            let width = 0.9 + ((column + row) % 3) as f32 * 0.2;
            let model = Path::new(BUILDING_MODELS[(column * 3 + row * 5) % BUILDING_MODELS.len()]);
            add_box(
                engine,
                model,
                root,
                Vec3::new(
                    column as f32 * BLOCK_SPACING - CITY_OFFSET,
                    height * 0.5,
                    row as f32 * BLOCK_SPACING - CITY_OFFSET,
                ),
                Vec3::new(width, height, width),
            )?;
        }
    }
    Ok(())
}

fn camera_at(elapsed: f32) -> ViewpointState {
    let angle = elapsed.rem_euclid(TOUR_SECONDS) * TAU / TOUR_SECONDS;
    let radius = 18.0 + 8.0 * (2.0 * angle).cos();
    let position = Vec3::new(
        radius * angle.sin(),
        11.0 + 4.0 * angle.sin(),
        radius * angle.cos(),
    );
    ViewpointState {
        position,
        direction: (Vec3::new(0.0, 2.0, 0.0) - position).normalize_or_zero(),
        fov_y: 60.0_f32.to_radians(),
    }
}

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    build_city(engine)?;
    let mut elapsed = 0.0_f32;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |scene, time| {
            elapsed = (elapsed + time.time_since_last_frame).rem_euclid(TOUR_SECONDS);
            scene.set_viewpoint(camera_at(elapsed))?;
            let angle = elapsed * TAU / TOUR_SECONDS;
            scene.set_lighting(SceneLighting {
                ambient: Vec3::splat(0.23),
                directional: DirectionalLight {
                    direction: Vec3::new(angle.cos(), -1.2, angle.sin()),
                    color: Vec3::new(0.9, 0.78, 0.65),
                },
            })
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!(
        "Flying around a city of {} buildings. Close the window to quit.",
        CITY_SIDE * CITY_SIDE
    );
    engine.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 1280, 720))?;
    let result = run_scene(&mut engine);
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

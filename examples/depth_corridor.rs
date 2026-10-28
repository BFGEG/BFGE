use std::error::Error;
use std::f32::consts::{PI, TAU};
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineEvent, SceneLighting, SceneNodeId, Transform,
    ViewpointState, WindowDescriptor,
};

const WINDOW_TITLE: &str = "BFGE depth corridor";
const PILLAR_MODELS: [&str; 2] = [
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/cube_coral.obj"
    ),
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_teal.obj"),
];
const BEAM_MODELS: [&str; 2] = [
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube_gold.obj"),
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/cube_violet.obj"
    ),
];
const FLOOR_MODEL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/cube_slate.obj"
);
const ARCH_COUNT: usize = 30;
const ARCH_SPACING: f32 = 2.8;
const START_Z: f32 = 6.0;
const END_Z: f32 = -((ARCH_COUNT - 1) as f32 * ARCH_SPACING) - 6.0;
const TRAVEL_SECONDS: f32 = 18.0;
const TURN_SECONDS: f32 = 2.0;
const CYCLE_SECONDS: f32 = 2.0 * (TRAVEL_SECONDS + TURN_SECONDS);

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

fn build_corridor(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let root = engine.scene().root_node();
    add_box(
        engine,
        Path::new(FLOOR_MODEL),
        root,
        Vec3::new(0.0, -0.2, (START_Z + END_Z) * 0.5),
        Vec3::new(6.0, 0.2, START_Z - END_Z + 2.0),
    )?;
    for arch in 0..ARCH_COUNT {
        let z = -(arch as f32) * ARCH_SPACING;
        for x in [-2.4, 2.4] {
            add_box(
                engine,
                Path::new(PILLAR_MODELS[arch % PILLAR_MODELS.len()]),
                root,
                Vec3::new(x, 1.8, z),
                Vec3::new(0.45, 3.6, 0.45),
            )?;
        }
        add_box(
            engine,
            Path::new(BEAM_MODELS[arch % BEAM_MODELS.len()]),
            root,
            Vec3::new(0.0, 3.75, z),
            Vec3::new(5.2, 0.45, 0.45),
        )?;
    }
    Ok(())
}

fn camera_at(elapsed: f32) -> ViewpointState {
    let elapsed = elapsed.rem_euclid(CYCLE_SECONDS);
    let (z, yaw) = if elapsed < TRAVEL_SECONDS {
        let progress = elapsed / TRAVEL_SECONDS;
        (START_Z + (END_Z - START_Z) * progress, 0.0)
    } else if elapsed < TRAVEL_SECONDS + TURN_SECONDS {
        let progress = (elapsed - TRAVEL_SECONDS) / TURN_SECONDS;
        (END_Z, PI * progress)
    } else if elapsed < 2.0 * TRAVEL_SECONDS + TURN_SECONDS {
        let progress = (elapsed - TRAVEL_SECONDS - TURN_SECONDS) / TRAVEL_SECONDS;
        (END_Z + (START_Z - END_Z) * progress, PI)
    } else {
        let progress = (elapsed - 2.0 * TRAVEL_SECONDS - TURN_SECONDS) / TURN_SECONDS;
        (START_Z, PI + PI * progress)
    };
    let cycle_angle = elapsed * TAU / CYCLE_SECONDS;
    ViewpointState {
        position: Vec3::new(
            0.25 * (4.0 * cycle_angle).sin(),
            1.6 + 0.08 * (6.0 * cycle_angle).sin(),
            z,
        ),
        direction: Vec3::new(yaw.sin(), -0.03, -yaw.cos()).normalize_or_zero(),
        fov_y: 65.0_f32.to_radians(),
    }
}

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    build_corridor(engine)?;
    let mut elapsed = 0.0_f32;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            let scene = context.scene();
            elapsed = (elapsed + time.time_since_last_frame).rem_euclid(CYCLE_SECONDS);
            scene.set_viewpoint(camera_at(elapsed))?;
            let angle = elapsed * TAU / CYCLE_SECONDS;
            scene
                .set_lighting(SceneLighting {
                    ambient: Vec3::splat(0.2),
                    directional: DirectionalLight {
                        direction: Vec3::new(0.5 * angle.sin(), -1.0, -0.6 * angle.cos()),
                        color: Vec3::new(0.95, 0.8, 0.6),
                    },
                })
                .map_err(Into::into)
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!("Flying through {ARCH_COUNT} arches and back. Close the window to quit.");
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

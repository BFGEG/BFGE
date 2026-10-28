use std::error::Error;
use std::f32::consts::TAU;
use std::path::Path;

use BFGE::math::{Quat, Vec3};
use BFGE::{
    DirectionalLight, Engine, EngineApi, EngineEvent, SceneError, SceneLighting, SceneNodeId,
    Transform, ViewpointState, WindowDescriptor,
};

const WINDOW_TITLE: &str = "BFGE showcase";
const CUBE_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");
const GRID_SIDE: usize = 10;
const GRID_COUNT: usize = GRID_SIDE * GRID_SIDE * GRID_SIDE;
const GRID_SPACING: f32 = 2.0;
const GRID_HALF_EXTENT: f32 = (GRID_SIDE as f32 - 1.0) * GRID_SPACING * 0.5;
const HERO_CENTER: Vec3 = Vec3::new(0.0, 0.0, -30.0);
const HERO_PARTS: [(Vec3, f32); 5] = [
    (Vec3::ZERO, 1.8),
    (Vec3::new(-2.5, 0.0, 0.0), 0.9),
    (Vec3::new(2.5, 0.0, 0.0), 0.9),
    (Vec3::new(0.0, 2.5, 0.0), 0.9),
    (Vec3::new(0.0, -2.5, 0.0), 0.9),
];
const ORBIT_SECONDS: f32 = 12.0;
const FLY_SECONDS: f32 = 10.0;
const HERO_SECONDS: f32 = 10.0;
const RETURN_SECONDS: f32 = 4.0;
const CYCLE_SECONDS: f32 = ORBIT_SECONDS + FLY_SECONDS + HERO_SECONDS + RETURN_SECONDS;

fn grid_position(index: usize) -> Vec3 {
    let x = index % GRID_SIDE;
    let y = index / GRID_SIDE % GRID_SIDE;
    let z = index / (GRID_SIDE * GRID_SIDE);
    Vec3::new(x as f32, y as f32, z as f32) * GRID_SPACING - Vec3::splat(GRID_HALF_EXTENT)
}

fn add_node(
    engine: &mut Engine,
    parent: SceneNodeId,
    transform: Transform,
) -> Result<SceneNodeId, SceneError> {
    let node = engine.scene().create_node(parent)?;
    engine.scene().set_node_transform(node, transform)?;
    Ok(node)
}

fn build_scene(engine: &mut Engine) -> Result<SceneNodeId, Box<dyn Error>> {
    let root = engine.scene().root_node();
    let model = Path::new(CUBE_MODEL);
    for index in 0..GRID_COUNT {
        let node = add_node(
            engine,
            root,
            Transform {
                translation: grid_position(index),
                rotation: Quat::from_rotation_y((index % 5) as f32 * 0.2),
                scale: Vec3::splat(0.7),
            },
        )?;
        engine.load_model(model, node)?;
    }

    let hero = add_node(engine, root, Transform::from_translation(HERO_CENTER))?;
    for (translation, size) in HERO_PARTS {
        let node = add_node(
            engine,
            hero,
            Transform {
                translation,
                scale: Vec3::splat(size),
                ..Transform::identity()
            },
        )?;
        engine.load_model(model, node)?;
    }
    let platform = add_node(
        engine,
        root,
        Transform {
            translation: HERO_CENTER + Vec3::new(0.0, -3.3, 0.0),
            scale: Vec3::new(8.0, 0.2, 8.0),
            ..Transform::identity()
        },
    )?;
    engine.load_model(model, platform)?;
    Ok(hero)
}

fn camera_at(elapsed: f32) -> ViewpointState {
    let elapsed = elapsed.rem_euclid(CYCLE_SECONDS);
    let (position, target) = if elapsed < ORBIT_SECONDS {
        let progress = elapsed / ORBIT_SECONDS;
        let angle = progress * TAU;
        (
            Vec3::new(
                30.0 * angle.sin(),
                8.0 * (1.0 - progress),
                30.0 * angle.cos(),
            ),
            Vec3::ZERO,
        )
    } else if elapsed < ORBIT_SECONDS + FLY_SECONDS {
        let progress = (elapsed - ORBIT_SECONDS) / FLY_SECONDS;
        (Vec3::new(0.0, 0.0, 30.0 - 53.0 * progress), HERO_CENTER)
    } else if elapsed < ORBIT_SECONDS + FLY_SECONDS + HERO_SECONDS {
        let progress = (elapsed - ORBIT_SECONDS - FLY_SECONDS) / HERO_SECONDS;
        let angle = progress * TAU;
        (
            HERO_CENTER + Vec3::new(7.0 * angle.sin(), 3.0 * progress, 7.0 * angle.cos()),
            HERO_CENTER,
        )
    } else {
        let progress = (elapsed - ORBIT_SECONDS - FLY_SECONDS - HERO_SECONDS) / RETURN_SECONDS;
        (
            Vec3::new(0.0, 3.0 + 5.0 * progress, -23.0 + 53.0 * progress),
            HERO_CENTER.lerp(Vec3::ZERO, progress),
        )
    };
    ViewpointState {
        position,
        direction: (target - position).normalize_or_zero(),
        fov_y: 60.0_f32.to_radians(),
    }
}

fn lighting_at(elapsed: f32) -> SceneLighting {
    let angle = elapsed * TAU / 14.0;
    let warmth = 0.5 + 0.5 * (elapsed * 0.5).sin();
    SceneLighting {
        ambient: Vec3::splat(0.22),
        directional: DirectionalLight {
            direction: Vec3::new(angle.cos(), -1.2, angle.sin()),
            color: Vec3::new(0.6 + 0.4 * warmth, 0.75, 1.0 - 0.45 * warmth),
        },
    }
}

fn run_scene(engine: &mut Engine) -> Result<(), Box<dyn Error>> {
    let hero = build_scene(engine)?;
    let mut elapsed = 0.0_f32;
    engine.add_event(EngineEvent {
        frame_started: Box::new(move |context, time| {
            let scene = context.scene();
            elapsed = (elapsed + time.time_since_last_frame).rem_euclid(CYCLE_SECONDS);
            scene.set_viewpoint(camera_at(elapsed))?;
            scene.set_lighting(lighting_at(elapsed))?;
            scene
                .set_node_transform(
                    hero,
                    Transform {
                        translation: HERO_CENTER,
                        rotation: Quat::from_rotation_y(elapsed * 0.8),
                        scale: Vec3::ONE,
                    },
                )
                .map_err(Into::into)
        }),
        frame_ended: Box::new(|_, _| Ok(())),
    })?;
    println!(
        "{} objects. Camera: grid orbit, fly-through, rotating group, return. Close the window to quit.",
        GRID_COUNT + HERO_PARTS.len() + 1
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_positions_cover_a_centered_ten_by_ten_by_ten_volume() {
        assert_eq!(grid_position(0), Vec3::splat(-GRID_HALF_EXTENT));
        assert_eq!(grid_position(GRID_COUNT - 1), Vec3::splat(GRID_HALF_EXTENT));
    }

    #[test]
    fn camera_path_joins_each_phase() {
        for boundary in [
            ORBIT_SECONDS,
            ORBIT_SECONDS + FLY_SECONDS,
            ORBIT_SECONDS + FLY_SECONDS + HERO_SECONDS,
            CYCLE_SECONDS,
        ] {
            let before = camera_at(boundary - 0.0001);
            let after = camera_at(boundary);
            assert!(before.position.distance(after.position) < 0.01);
            assert!(before.direction.distance(after.direction) < 0.01);
        }
    }

    #[test]
    fn scene_builds_without_a_graphics_session() {
        let mut engine = Engine::new();

        build_scene(&mut engine).expect("the bundled OBJ and scene hierarchy are valid");
    }
}

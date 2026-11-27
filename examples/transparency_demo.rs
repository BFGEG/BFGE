//! Transparent marker with a texture fade and an opaque object inside.
use BFGE::math::Vec3;
use BFGE::{
    Engine, EngineApi, EngineEvent, KeyCode, SceneLighting, Transform, ViewpointState,
    WindowDescriptor,
};
use std::{
    error::Error,
    f32::consts::TAU,
    fs,
    path::Path,
    time::{Duration, Instant},
};

const WINDOW_TITLE: &str = "BFGE — alpha transparency";
const CUBE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/cube.obj");
const AUTO_FLAG: &str = "--auto";
const MARKER: &str = "marker.obj";
const MATERIAL: &str = "marker.mtl";
const TEXTURE: &str = "fade.png";

fn main() -> Result<(), Box<dyn Error>> {
    let automatic = std::env::args().any(|arg| arg == AUTO_FLAG);
    let assets = tempfile::tempdir()?;
    let texture = image::RgbaImage::from_fn(1, 128, |_, y| {
        image::Rgba([40, 230, 190, (210 * y / 127) as u8])
    });
    texture.save(assets.path().join(TEXTURE))?;
    fs::write(
        assets.path().join(MATERIAL),
        format!("newmtl marker\nKd 1 1 1\nd 0.65\nmap_Kd {TEXTURE}\n"),
    )?;
    let mut model = format!("mtllib {MATERIAL}\n");
    for index in 0..48 {
        let a = TAU * index as f32 / 48.0;
        let b = TAU * (index + 1) as f32 / 48.0;
        model.push_str(&format!("o panel{index}\nusemtl marker\n"));
        for (angle, y) in [(a, 0.0), (b, 0.0), (b, 1.5), (a, 1.5)] {
            model.push_str(&format!(
                "v {} {y} {}\n",
                1.4 * angle.cos(),
                1.4 * angle.sin()
            ));
        }
        model.push_str("vt 0 0.00390625\nvt 1 0.00390625\nvt 1 0.99609375\nvt 0 0.99609375\n");
        let n = index * 4 + 1;
        model.push_str(&format!(
            "f {n}/{n} {}/{} {}/{} {}/{}\n",
            n + 1,
            n + 1,
            n + 2,
            n + 2,
            n + 3,
            n + 3
        ));
    }
    fs::write(assets.path().join(MARKER), model)?;
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 960, 640))?;
    let result = (|| -> Result<(), Box<dyn Error>> {
        let root = engine.scene().root_node();
        for (position, size, color) in [
            (
                Vec3::new(0.0, -0.15, 0.0),
                Vec3::new(8.0, 0.2, 8.0),
                [0.15, 0.19, 0.25],
            ),
            (
                Vec3::new(0.0, 0.45, 0.0),
                Vec3::new(0.8, 1.0, 0.8),
                [1.0, 0.3, 0.08],
            ),
            (
                Vec3::new(1.8, 0.45, -2.0),
                Vec3::new(1.0, 1.0, 1.0),
                [0.7, 0.7, 0.8],
            ),
        ] {
            let node = engine.scene().create_node(root)?;
            engine.scene().set_node_transform(
                node,
                Transform {
                    translation: position,
                    scale: size,
                    ..Transform::default()
                },
            )?;
            let object = engine.load_model(Path::new(CUBE), node)?;
            engine.scene().set_object_color(object, color)?;
        }
        engine.load_model(&assets.path().join(MARKER), root)?;
        let lighting = SceneLighting {
            ambient: Vec3::splat(0.7),
            ..SceneLighting::default()
        };
        engine.scene().set_lighting(lighting)?;
        let started = Instant::now();
        engine.add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                let elapsed = started.elapsed().as_secs_f32();
                let angle = elapsed * 0.22;
                let position = Vec3::new(5.0 * angle.sin(), 3.0, 5.0 * angle.cos());
                context.scene().set_viewpoint(ViewpointState {
                    position,
                    direction: (Vec3::new(0.0, 0.5, 0.0) - position).normalize(),
                    roll: 0.0,
                    fov_y: 55.0_f32.to_radians(),
                })?;
                if context.input().is_key_pressed(KeyCode::Escape)
                    || (automatic && started.elapsed() > Duration::from_secs(3))
                {
                    context.request_exit();
                }
                Ok(())
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })?;
        engine.run()?;
        Ok(())
    })();
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

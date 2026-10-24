use std::error::Error;
use std::path::Path;

use BFGE::stress::{BenchmarkMode, run};

const SCENE_ONLY_MODE: &str = "scene-only";
const WINDOW_MODE: &str = "window";

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let [_, mode, config, output] = args.as_slice() else {
        return Err("usage: bfge-stress <scene-only|window> <config.json> <output.json>".into());
    };
    let mode = match mode.as_str() {
        SCENE_ONLY_MODE => BenchmarkMode::SceneOnly,
        WINDOW_MODE => BenchmarkMode::Window,
        _ => return Err("mode must be scene-only or window".into()),
    };
    run(mode, Path::new(config), Path::new(output))?;
    Ok(())
}

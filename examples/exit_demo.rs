//! Press Escape to return from the engine loop and shut the session down.

use std::error::Error;
use std::time::{Duration, Instant};

use BFGE::{Engine, EngineApi, EngineEvent, KeyCode, WindowDescriptor};

const WINDOW_TITLE: &str = "BFGE — Escape to exit";
const AUTO_FLAG: &str = "--auto";

fn main() -> Result<(), Box<dyn Error>> {
    let automatic = std::env::args().any(|argument| argument == AUTO_FLAG);
    let started = Instant::now();
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 640, 360))?;
    let result = engine
        .add_event(EngineEvent {
            frame_started: Box::new(move |context, _| {
                if context.input().is_key_pressed(KeyCode::Escape)
                    || (automatic && started.elapsed() >= Duration::from_secs(1))
                {
                    context.request_exit();
                }
                Ok(())
            }),
            frame_ended: Box::new(|_, _| Ok(())),
        })
        .and_then(|()| engine.run());
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

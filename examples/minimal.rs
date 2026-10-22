use BFGE::{Engine, EngineApi, EngineError, WindowDescriptor};

const WINDOW_TITLE: &str = "BFGE minimal";

fn main() -> Result<(), EngineError> {
    let mut engine = Engine::new();
    engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 800, 600))?;
    let result = engine.run();
    let shutdown = engine.shutdown();
    result?;
    shutdown?;
    Ok(())
}

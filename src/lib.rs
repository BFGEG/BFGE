//! FREAK Engine (BFGE) - a Rust graphics engine.
//!
//! Public contract - `FREAK_Engine_Public_API.md`;
//! internal skeleton - `docs/FREAK_Engine_Architecture.md`.

#![warn(missing_docs)]
// `BFGE` is a product name, not a variable/module; the crate must not be renamed.
#![allow(non_snake_case)]

pub mod math;
pub mod render;
pub mod resources;

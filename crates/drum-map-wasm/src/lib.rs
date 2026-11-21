//! wasm-bindgen bindings exposing `drum-map` to the JS/TS frontend.
//!
//! Filled in during milestone M3; exists now so the workspace compiles.

use wasm_bindgen::prelude::*;

/// Placeholder binding so the crate links; real API arrives in M3.
#[wasm_bindgen]
pub fn drum_map_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

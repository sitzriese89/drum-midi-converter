//! The core crate itself: version, for the UI footer/title.

use wasm_bindgen::prelude::*;

/// The version of the `drum-map` core (the workspace version, shared with
/// the core crate and the CLI).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

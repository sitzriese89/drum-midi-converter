//! The built-in map catalog (`drum_map::catalog`).

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// One entry of the built-in catalog.
#[derive(Serialize)]
pub struct MapInfo {
    /// Stable lowercase kebab-case id: `ggd-invasion`, `gm`.
    pub id: String,
    /// Display name, e.g. `GetGood Drums Invasion`.
    pub name: String,
}

/// All built-in maps, in catalog (declaration) order.
#[wasm_bindgen]
pub fn list_maps() -> Result<JsValue, JsError> {
    to_js(
        &drum_map::catalog()?
            .into_iter()
            .map(|map| MapInfo {
                id: map.id,
                name: map.name,
            })
            .collect::<Vec<_>>(),
    )
}

//! Detection hint bindings (`drum_map::detect`, `drum_map::midi::used_drum_notes`).
//!
//! Detection is a hint, never a decision (ADR-003): the UI pre-selects a
//! source map from these candidates and warns on a mismatch, but never
//! silently converts.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// One ranked detection candidate.
#[derive(Serialize)]
pub struct Candidate {
    /// Candidate map id.
    pub map_id: String,
    /// Confidence in `0.0..=1.0` (the plain coverage fraction).
    pub confidence: f64,
}

/// Rank all catalog maps against the drum notes used in `midi_bytes`.
///
/// Returns candidates sorted by descending confidence; empty when the file
/// has no drum notes. Throws a JS `Error` for invalid SMF data.
#[wasm_bindgen]
pub fn detect_hints(midi_bytes: &[u8]) -> Result<JsValue, JsError> {
    let used = drum_map::midi::used_drum_notes(midi_bytes).map_err(js_error)?;
    let catalog = drum_map::catalog().map_err(js_error)?;
    let candidates = drum_map::detect::detect(&used, &catalog);
    to_js(
        &candidates
            .iter()
            .map(|c| Candidate {
                map_id: c.map_id.clone(),
                confidence: c.confidence,
            })
            .collect::<Vec<_>>(),
    )
}

/// How well one candidate map covers the file's drum notes: the fraction of
/// used notes the map contains, in `0.0..=1.0`. `None` when the file has no
/// drum notes or the id is unknown — the UI shows no mismatch warning for
/// either (ADR-003: the user decides).
#[wasm_bindgen]
pub fn source_coverage(midi_bytes: &[u8], map_id: &str) -> Result<Option<f64>, JsError> {
    let used = drum_map::midi::used_drum_notes(midi_bytes).map_err(js_error)?;
    if used.is_empty() {
        return Ok(None);
    }
    let Some(map) = drum_map::find_map(map_id).map_err(js_error)? else {
        return Ok(None);
    };
    let hits = used.iter().filter(|&&n| map.note(n).is_some()).count();
    Ok(Some(hits as f64 / used.len() as f64))
}

/// Convert a core error into a JS `Error`.
fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

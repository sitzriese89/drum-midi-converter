//! Full conversion binding (`drum_map::midi::convert_by_id`).
//!
//! Input: SMF bytes, source/target map ids and per-note overrides for
//! unmapped notes (ADR-004). Output: the converted SMF bytes plus the
//! report (SPEC §4.2/§4.3). All logic stays in the core; this module only
//! moves values across the WASM boundary.

use std::collections::HashMap;

use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// What to do with one unmapped note (ADR-004), as the UI sends it:
/// `{ "kind": "keep" }`, `{ "kind": "drop" }` or `{ "kind": "reassign",
/// "note": 60 }`, keyed by source note number.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum OverrideInput {
    Keep,
    Drop,
    Reassign { note: u8 },
}

/// The conversion result: converted SMF bytes and the report.
#[derive(serde::Serialize)]
pub struct ConvertResult {
    /// Converted SMF bytes (a `Uint8Array` on the JS side).
    pub midi: Box<[u8]>,
    /// The conversion report (one row per used note).
    pub report: drum_map::Report,
}

/// Convert `midi_bytes` from map `from_id` to map `to_id`.
///
/// `per_note` is `null` or an object `{ "56": { "kind": "drop" }, … }` of
/// per-note overrides; notes without an entry keep their note number
/// (ADR-004). Returns `{ midi: Uint8Array, report: Report }`. Throws a JS
/// `Error` for invalid SMF data or an unknown map id.
#[wasm_bindgen]
pub fn convert_midi(
    midi_bytes: &[u8],
    from_id: &str,
    to_id: &str,
    per_note: Option<JsValue>,
) -> Result<JsValue, JsError> {
    let mut options = drum_map::ConvertOptions::default();
    if let Some(value) = per_note.filter(|v| !v.is_null() && !v.is_undefined()) {
        let inputs: HashMap<String, OverrideInput> =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
        options.per_note = inputs
            .into_iter()
            .map(|(key, input)| {
                // JS object keys are strings; the note number must parse as 0..=127.
                let note: u8 = key
                    .parse()
                    .map_err(|_| JsError::new(&format!("invalid note number {key:?}")))?;
                let resolution = match input {
                    OverrideInput::Keep => drum_map::Resolution::Keep,
                    OverrideInput::Drop => drum_map::Resolution::Drop,
                    OverrideInput::Reassign { note } => drum_map::Resolution::Reassign(note),
                };
                Ok((note, resolution))
            })
            .collect::<Result<HashMap<u8, drum_map::Resolution>, JsError>>()?;
    }
    let (midi, report) =
        drum_map::convert_by_id(midi_bytes, from_id, to_id, &options).map_err(|e| {
            JsError::new(&match e {
                drum_map::midi::MidiError::UnknownMap { id } => {
                    format!("unknown map id \"{id}\" in the embedded catalog")
                }
                other => other.to_string(),
            })
        })?;
    to_js(&ConvertResult {
        midi: midi.into_boxed_slice(),
        report,
    })
}

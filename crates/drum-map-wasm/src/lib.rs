//! WebAssembly bindings exposing the `drum-map` crate to the frontend.
//!
//! A thin layer: one module per core area, no conversion logic here
//! (ADR-002). The app builds it with `wasm-pack` (`pnpm --dir app wasm`).

use wasm_bindgen::prelude::*;

mod catalog;
mod convert;
mod detect;
mod meta;

pub use catalog::list_maps;
pub use convert::convert_midi;
pub use detect::{detect_hints, source_coverage};
pub use meta::version;

/// Serialize a value into a plain, JSON-compatible JS value.
fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| JsError::new(&error.to_string()))
}

/// The TS types of the values serialized with `to_js` above: plain objects
/// matching the Rust structs field for field (serde renames to camelCase is
/// NOT used — the field names match the core's report model, snake_case).
#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = r#"
/** One entry of the built-in catalog. */
export interface MapInfo {
  /** Stable lowercase kebab-case id: `ggd-invasion`, `gm`. */
  id: string;
  /** Display name, e.g. `GetGood Drums Invasion`. */
  name: string;
}

/** One ranked detection candidate (ADR-003). */
export interface Candidate {
  /** Candidate map id. */
  map_id: string;
  /** Confidence in `0.0..=1.0` (the plain coverage fraction). */
  confidence: number;
}

/** One row of the conversion report: one used source note (SPEC 4.3). */
export interface NoteRow {
  /** Source MIDI note number. */
  source_note: number;
  /** Source note display name, e.g. `Kick In`. */
  source_name: string;
  /** Number of note-on hits for this note in the file. */
  hit_count: number;
  /** Mapped target MIDI note number, or null when unmapped. */
  target_note: number | null;
  /** Target note display name, or null when unmapped. */
  target_name: string | null;
  /** True when a fallback rule fired for this note (LOSSY badge). */
  lossy: boolean;
  /** True when the note had no mapping entry (ADR-004). */
  unmapped: boolean;
}

/** Full conversion report. */
export interface Report {
  /** One row per used source note, sorted by source note number. */
  rows: NoteRow[];
}

/** The conversion result: converted SMF bytes and the report. */
export interface ConvertResult {
  /** Converted SMF bytes (a Uint8Array on the JS side). */
  midi: Uint8Array;
  /** The conversion report. */
  report: Report;
}
"#;

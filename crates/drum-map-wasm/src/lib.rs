//! WebAssembly bindings exposing the `drum-map` crate to the frontend.
//!
//! A thin layer: one module per core area, no conversion logic here
//! (ADR-002). The app builds it with `wasm-pack` (`pnpm --dir app wasm`).

use wasm_bindgen::prelude::*;

mod catalog;
mod convert;
mod detect;
mod meta;
mod user_maps;

pub use catalog::{list_maps, map_notes};
pub use convert::convert_midi;
pub use detect::{detect_hints, source_coverage};
pub use meta::version;
pub use user_maps::{builtin_map, map_to_toml, parse_map_toml, validate_map};

/// Serialize a value into a plain, JSON-compatible JS value.
fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| JsError::new(&error.to_string()))
}

/// The TS types of the values serialized with `to_js` above: plain objects
/// matching the Rust structs field for field (serde renames to camelCase is
/// NOT used — the field names match the core's report model, snake_case).
/// NOTE: these interfaces are hand-written mirrors of the Rust structs and
/// have no compile-time link — they MUST be kept in sync by hand (a
/// generated-types step is deferred; see docs/roadmap.md).
#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = r#"
/** One entry of the built-in catalog. */
export interface MapInfo {
  /** Stable lowercase kebab-case id: `ggd-invasion`, `gm`. */
  id: string;
  /** Display name, e.g. `GetGood Drums Invasion`. */
  name: string;
}

/** One note of a map's vocabulary: MIDI note number and display name. */
export interface NoteName {
  /** MIDI note number (0-127). */
  note: number;
  /** Display name, e.g. `Kick In`. */
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

/** One note of a map under construction (map editor, SPEC 3.4). */
export interface MapNote {
  /** MIDI note number (0-127). */
  note: number;
  /** Display name, e.g. `Kick In`. */
  name: string;
  /** Semantic tag text (ADR-001 vocabulary), e.g. `kick.1`. */
  tag: string;
}

/** One fallback rule of a map under construction. */
export interface MapFallbackRule {
  /** The source articulation that is being collapsed. */
  from: string;
  /** The less specific articulation to try instead. */
  to: string;
}

/** A map under construction, as the editor keeps and sends it in. */
export interface MapInput {
  /** Stable id, also the file name in the app-data folder. */
  id: string;
  /** Display name. */
  name: string;
  /** Vendor. */
  vendor: string;
  /** Free-form description, null when empty. */
  description: string | null;
  /** Note entries. */
  notes: MapNote[];
  /** Documented fallback rules; null when the map has none. */
  fallback: MapFallbackRule[] | null;
}

/** A validated map: notes sorted ascending by note number. */
export interface MapFull {
  /** Stable id, also the file name in the app-data folder. */
  id: string;
  /** Display name. */
  name: string;
  /** Vendor. */
  vendor: string;
  /** Free-form description, null when empty. */
  description: string | null;
  /** Note entries, sorted ascending by note number. */
  notes: MapNote[];
  /** Documented fallback rules; null when the map has none. */
  fallback: MapFallbackRule[] | null;
}

/** One validation problem, naming the field it concerns. */
export interface MapIssue {
  /** The field: `id`, `notes.36.tag`, `fallback.rules[1].from`, `toml` for a parse error. */
  field: string;
  /** The full problem message. */
  message: string;
}

/** What parsing or validating a map produced; `map` is null when invalid. */
export interface MapParseResult {
  /** The validated map, or null when there are issues. */
  map: MapFull | null;
  /** Every problem found, field-by-field where possible. */
  issues: MapIssue[];
}
"#;

/// Shared SMF fixtures for the binding-layer unit tests. The bindings'
/// plain-Rust `*_inner` functions run natively under `cargo test` (the
/// crate is an rlib), so the fixtures mirror the core's `midi.rs` tests and
/// are built with `midly` here too.
#[cfg(test)]
pub(crate) mod test_support {
    use midly::{
        MidiMessage, Smf, Track, TrackEvent, TrackEventKind,
        num::{u4, u7, u15, u28},
    };

    fn drum_channel() -> u4 {
        u4::new(9)
    }

    fn note_on(key: u8, delta: u32) -> TrackEvent<'static> {
        TrackEvent {
            delta: u28::new(delta),
            kind: TrackEventKind::Midi {
                channel: drum_channel(),
                message: MidiMessage::NoteOn {
                    key: u7::new(key),
                    vel: u7::new(100),
                },
            },
        }
    }

    fn note_off(key: u8, delta: u32) -> TrackEvent<'static> {
        TrackEvent {
            delta: u28::new(delta),
            kind: TrackEventKind::Midi {
                channel: drum_channel(),
                message: MidiMessage::NoteOff {
                    key: u7::new(key),
                    vel: u7::new(64),
                },
            },
        }
    }

    fn write(header: midly::Header, tracks: Vec<Track<'static>>) -> Vec<u8> {
        let mut bytes = Vec::new();
        Smf { header, tracks }.write(&mut bytes).unwrap();
        bytes
    }

    /// Format-0 SMF: one kick hit (36) plus a cowbell hit (56) on channel 10.
    pub(crate) fn drum_smf() -> Vec<u8> {
        let header = midly::Header::new(
            midly::Format::SingleTrack,
            midly::Timing::Metrical(u15::new(480)),
        );
        let track: Track = vec![
            note_on(36, 0),
            note_off(36, 240),
            note_on(56, 240),
            note_off(56, 240),
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            },
        ];
        write(header, vec![track])
    }

    /// Format-0 SMF whose only track is a meta event: no drum notes at all.
    pub(crate) fn no_drum_smf() -> Vec<u8> {
        let header = midly::Header::new(
            midly::Format::SingleTrack,
            midly::Timing::Metrical(u15::new(480)),
        );
        let track: Track = vec![TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
        }];
        write(header, vec![track])
    }
}

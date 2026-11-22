//! Full conversion binding (`drum_map::midi::convert_by_id`).
//!
//! Input: SMF bytes, source/target map ids and per-note overrides for
//! unmapped AND lossy notes (ADR-004, SPEC §4.3). Output: the converted SMF
//! bytes plus the report (SPEC §4.2/§4.3). All logic stays in the core;
//! this module only moves values across the WASM boundary. The
//! plain-Rust `*_inner` function is what the unit tests exercise (it runs
//! natively under `cargo test`).

use std::collections::HashMap;

use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// What to do with one unmapped note (ADR-004), as the UI sends it:
/// `{ "kind": "keep" }`, `{ "kind": "drop" }` or `{ "kind": "reassign",
/// "note": 60 }`, keyed by source note number.
#[derive(Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum OverrideInput {
    Keep,
    Drop,
    Reassign { note: u8 },
}

/// The conversion result: converted SMF bytes and the report.
#[derive(serde::Serialize, Debug)]
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
    let inputs: Option<HashMap<String, OverrideInput>> = match per_note {
        Some(value) if !value.is_null() && !value.is_undefined() => {
            Some(serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?)
        }
        _ => None,
    };
    let result = convert_midi_inner(midi_bytes, from_id, to_id, inputs.as_ref())
        .map_err(|e| JsError::new(&e))?;
    to_js(&result)
}

fn convert_midi_inner(
    midi_bytes: &[u8],
    from_id: &str,
    to_id: &str,
    per_note: Option<&HashMap<String, OverrideInput>>,
) -> Result<ConvertResult, String> {
    let mut options = drum_map::ConvertOptions::default();
    if let Some(inputs) = per_note {
        options.per_note = inputs
            .iter()
            .map(|(key, input)| {
                // JS object keys are strings; the note number must parse as
                // 0..=127.
                let note = parse_note_key(key)?;
                let resolution = match input {
                    OverrideInput::Keep => drum_map::Resolution::Keep,
                    OverrideInput::Drop => drum_map::Resolution::Drop,
                    OverrideInput::Reassign { note } => drum_map::Resolution::Reassign(*note),
                };
                Ok((note, resolution))
            })
            .collect::<Result<HashMap<u8, drum_map::Resolution>, String>>()?;
    }
    let (midi, report) =
        drum_map::convert_by_id(midi_bytes, from_id, to_id, &options).map_err(|e| match e {
            drum_map::midi::MidiError::UnknownMap { id } => {
                format!("unknown map id \"{id}\" in the embedded catalog")
            }
            other => other.to_string(),
        })?;
    Ok(ConvertResult {
        midi: midi.into_boxed_slice(),
        report,
    })
}

/// Parse one per-note override key: the JS object key of a note number,
/// which must be an integer in `0..=127`.
fn parse_note_key(key: &str) -> Result<u8, String> {
    key.parse::<u8>()
        .ok()
        .filter(|note| *note <= 127)
        .ok_or_else(|| format!("invalid note number {key:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::drum_smf;

    fn overrides(pairs: &[(&str, OverrideInput)]) -> HashMap<String, OverrideInput> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn happy_path_converts_and_reports() {
        let result = convert_midi_inner(&drum_smf(), "gm", "ggd-invasion", None).unwrap();
        let kick = result
            .report
            .rows
            .iter()
            .find(|r| r.source_note == 36)
            .unwrap();
        assert_eq!(kick.target_note, Some(24));
        assert!(!kick.unmapped);
        // Output is valid SMF: the kick note-on became Invasion note 24.
        assert_eq!(result.midi[0], 0x4d); // "M"
        assert!(result.midi.contains(&24));
    }

    #[test]
    fn unknown_map_id_is_a_named_error() {
        let err = convert_midi_inner(&drum_smf(), "gm", "no-such-map", None).unwrap_err();
        assert!(err.contains("unknown map id \"no-such-map\""), "{err}");
        let err = convert_midi_inner(&drum_smf(), "no-such-map", "gm", None).unwrap_err();
        assert!(err.contains("no-such-map"), "{err}");
    }

    #[test]
    fn invalid_smf_is_an_error() {
        assert!(convert_midi_inner(&[0u8; 4], "gm", "ggd-invasion", None).is_err());
    }

    #[test]
    fn per_note_drop_override_applies() {
        let per_note = overrides(&[("56", OverrideInput::Drop)]);
        let result =
            convert_midi_inner(&drum_smf(), "gm", "ggd-invasion", Some(&per_note)).unwrap();
        let cowbell = result
            .report
            .rows
            .iter()
            .find(|r| r.source_note == 56)
            .unwrap();
        assert!(cowbell.unmapped);
        assert_eq!(cowbell.target_note, None);
        assert!(!result.midi.contains(&56));
    }

    #[test]
    fn per_note_reassign_override_applies_to_a_mapped_note() {
        // 36 maps exactly (GM kick -> Invasion kick); an explicit override
        // must still win (SPEC §4.3: overrides for unmapped AND lossy rows).
        let per_note = overrides(&[("36", OverrideInput::Reassign { note: 60 })]);
        let result =
            convert_midi_inner(&drum_smf(), "gm", "ggd-invasion", Some(&per_note)).unwrap();
        let kick = result
            .report
            .rows
            .iter()
            .find(|r| r.source_note == 36)
            .unwrap();
        assert_eq!(kick.target_note, Some(60));
        assert!(result.midi.contains(&60));
        assert!(!result.midi.contains(&24));
    }

    #[test]
    fn note_key_parse() {
        assert_eq!(parse_note_key("56"), Ok(56));
        assert_eq!(parse_note_key("0"), Ok(0));
        assert_eq!(parse_note_key("127"), Ok(127));
        assert!(parse_note_key("abc").is_err());
        assert!(parse_note_key("-1").is_err());
        assert!(parse_note_key("").is_err());
        // Outside the MIDI range.
        assert!(parse_note_key("128").is_err());
        assert!(parse_note_key("256").is_err());
    }
}

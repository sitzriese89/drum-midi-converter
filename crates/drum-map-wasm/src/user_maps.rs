//! The user map editor (`drum_map` map building/validating, SPEC §3.4, Q6).
//!
//! The editor works on a structured `MapInput` (id, name, vendor, notes,
//! fallback rules); the core validates it field-by-field and serializes it to
//! TOML with the built-in schema. Importing parses TOML the same way and
//! surfaces parse problems instead of silently dropping notes.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// One note of a map under construction.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct MapNote {
    /// MIDI note number (0-127).
    pub note: u8,
    /// Display name, e.g. `Kick In`.
    pub name: String,
    /// Semantic tag text (ADR-001 vocabulary).
    pub tag: String,
}

/// One fallback rule of a map under construction.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct MapFallbackRule {
    /// The source articulation that is being collapsed.
    pub from: String,
    /// The less specific articulation to try instead.
    pub to: String,
}

/// A validated map, as the editor keeps it (notes sorted ascending).
#[derive(Serialize, Debug, Clone)]
pub struct MapFull {
    /// Stable id, also the file name in the app-data folder.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Vendor.
    pub vendor: String,
    /// Free-form description, null when empty.
    pub description: Option<String>,
    /// Note entries, sorted ascending by note number.
    pub notes: Vec<MapNote>,
    /// Documented fallback rules; null when the map has none.
    pub fallback: Option<Vec<MapFallbackRule>>,
}

/// One validation problem, naming the field it concerns. The core's type,
/// re-exported so the boundary has one `MapIssue`, not a parallel copy.
pub use drum_map::MapIssue;

/// What parsing or validating a map produced: a `MapFull` only when there are
/// no issues; the issues are shown in the editor, never silently ignored.
#[derive(Serialize, Debug, Clone)]
pub struct MapParseResult {
    /// The map, or null when it is invalid.
    pub map: Option<MapFull>,
    /// Every problem found, field-by-field where possible.
    pub issues: Vec<MapIssue>,
}

/// The map under construction as the editor sends it in.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MapInput {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub description: Option<String>,
    pub notes: Vec<MapNoteInput>,
    pub fallback: Option<Vec<MapFallbackRuleInput>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MapNoteInput {
    pub note: u8,
    pub name: String,
    pub tag: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MapFallbackRuleInput {
    pub from: String,
    pub to: String,
}

/// Validates a map under construction and returns it when valid.
#[wasm_bindgen]
pub fn validate_map(input: Option<JsValue>) -> Result<JsValue, JsError> {
    let input: MapInput =
        serde_wasm_bindgen::from_value(input.ok_or_else(|| JsError::new("missing map input"))?)
            .map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&validate_map_inner(input).map_err(|e| JsError::new(&e))?)
}

/// Serializes a valid map to TOML with the built-in schema. Throws when the
/// map is invalid: nothing is written from an invalid editor state.
#[wasm_bindgen]
pub fn map_to_toml(input: Option<JsValue>) -> Result<String, JsError> {
    let input: MapInput =
        serde_wasm_bindgen::from_value(input.ok_or_else(|| JsError::new("missing map input"))?)
            .map_err(|e| JsError::new(&e.to_string()))?;
    map_to_toml_inner(input).map_err(|e| JsError::new(&e))
}

/// Parses a user map's TOML (import, editing a stored map). Never throws:
/// a broken file comes back with the parse error in `issues` and a null map.
#[wasm_bindgen]
pub fn parse_map_toml(text: &str) -> Result<JsValue, JsError> {
    to_js(&parse_map_toml_inner(text))
}

/// A built-in map as a full, editable copy: the starting point for
/// "duplicate a built-in map" (built-ins themselves stay read-only).
/// Throws for an unknown map id.
#[wasm_bindgen]
pub fn builtin_map(map_id: &str) -> Result<JsValue, JsError> {
    to_js(&builtin_map_inner(map_id).map_err(|e| JsError::new(&e))?)
}

fn full_of(map: drum_map::DrumMap) -> MapFull {
    MapFull {
        id: map.id.clone(),
        name: map.name.clone(),
        vendor: map.vendor.clone(),
        description: map.description.clone(),
        notes: map
            .entries()
            .into_iter()
            .map(|(note, entry)| MapNote {
                note,
                name: entry.name.clone(),
                tag: entry.tag.to_string(),
            })
            .collect(),
        fallback: map.fallback.map(|f| {
            f.rules
                .into_iter()
                .map(|r| MapFallbackRule {
                    from: r.from.to_string(),
                    to: r.to.to_string(),
                })
                .collect()
        }),
    }
}

fn input_of(
    input: MapInput,
) -> (
    String,
    String,
    String,
    Option<String>,
    Vec<drum_map::NoteInput>,
    Vec<drum_map::FallbackRuleInput>,
) {
    (
        input.id,
        input.name,
        input.vendor,
        input.description,
        input
            .notes
            .into_iter()
            .map(|n| drum_map::NoteInput {
                note: n.note,
                name: n.name,
                tag: n.tag,
            })
            .collect(),
        input
            .fallback
            .unwrap_or_default()
            .into_iter()
            .map(|r| drum_map::FallbackRuleInput {
                from: r.from,
                to: r.to,
            })
            .collect(),
    )
}

fn build(input: MapInput) -> Result<drum_map::DrumMap, String> {
    let (id, name, vendor, description, notes, fallback) = input_of(input);
    drum_map::DrumMap::build(
        &id,
        &name,
        &vendor,
        description.as_deref(),
        &notes,
        &fallback,
    )
    .map_err(|issues| {
        issues
            .iter()
            .map(|i| format!("{}: {}", i.field, i.message))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn validate_map_inner(input: MapInput) -> Result<MapParseResult, String> {
    let (id, name, vendor, description, notes, fallback) = input_of(input);
    match drum_map::DrumMap::build(
        &id,
        &name,
        &vendor,
        description.as_deref(),
        &notes,
        &fallback,
    ) {
        Ok(map) => Ok(MapParseResult {
            map: Some(full_of(map)),
            issues: Vec::new(),
        }),
        Err(issues) => Ok(MapParseResult { map: None, issues }),
    }
}

fn map_to_toml_inner(input: MapInput) -> Result<String, String> {
    Ok(build(input)?.to_toml())
}

fn parse_map_toml_inner(text: &str) -> MapParseResult {
    match toml::from_str::<drum_map::DrumMap>(text) {
        Ok(map) => MapParseResult {
            map: Some(full_of(map)),
            issues: Vec::new(),
        },
        Err(error) => MapParseResult {
            map: None,
            issues: vec![MapIssue {
                field: "toml".into(),
                message: error.to_string(),
            }],
        },
    }
}

fn builtin_map_inner(map_id: &str) -> Result<MapFull, String> {
    let map = drum_map::find_map(map_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown map id \"{map_id}\" in the embedded catalog"))?;
    Ok(full_of(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MapInput {
        MapInput {
            id: "my-kit".into(),
            name: "My Kit".into(),
            vendor: "Me".into(),
            description: Some("Home kit.".into()),
            notes: vec![
                MapNoteInput {
                    note: 36,
                    name: "Kick".into(),
                    tag: "kick.1".into(),
                },
                MapNoteInput {
                    note: 42,
                    name: "Closed HH".into(),
                    tag: "hihat.closed.edge".into(),
                },
            ],
            fallback: Some(vec![MapFallbackRuleInput {
                from: "hihat.open.2".into(),
                to: "hihat.open".into(),
            }]),
        }
    }

    #[test]
    fn valid_input_validates_and_serializes() {
        let result = validate_map_inner(sample()).unwrap();
        let map = result.map.unwrap();
        assert!(result.issues.is_empty());
        assert_eq!(map.id, "my-kit");
        // Notes sorted ascending by number, whatever the editor's order.
        assert_eq!(
            map.notes.iter().map(|n| n.note).collect::<Vec<_>>(),
            vec![36, 42]
        );
        let toml_text = map_to_toml_inner(sample()).unwrap();
        // The saved TOML parses back to the same map (SPEC §3.4).
        let parsed = parse_map_toml_inner(&toml_text);
        assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
        let round = parsed.map.unwrap();
        assert_eq!(round.id, map.id);
        assert_eq!(round.notes, map.notes);
        assert_eq!(round.fallback, map.fallback);
        // Same layout as the built-in files: `[notes.36]` tables.
        assert!(toml_text.contains("[notes.36]"));
        assert!(toml_text.contains("tag = \"kick.1\""));
        assert!(toml_text.contains("[[fallback.rules]]"));
    }

    #[test]
    fn invalid_input_reports_the_problems_without_a_map() {
        let mut input = sample();
        input.id = "Bad Id".into();
        input.notes[0].tag = "Kick".into();
        let result = validate_map_inner(input).unwrap();
        assert!(result.map.is_none());
        let (fields, messages): (Vec<_>, Vec<_>) = result
            .issues
            .iter()
            .map(|i| (i.field.as_str(), i.message.as_str()))
            .unzip();
        assert!(fields.contains(&"id"), "{messages:?}");
        // Note fields are keyed by MIDI note number, not row index.
        assert!(fields.contains(&"notes.36.tag"), "{messages:?}");
    }

    #[test]
    fn a_broken_toml_comes_back_with_the_parse_error() {
        let result = parse_map_toml_inner("id = \"x\"\nname = ");
        assert!(result.map.is_none());
        assert_eq!(result.issues[0].field, "toml");
        let result = parse_map_toml_inner(
            "id = \"x\"\nname = \"X\"\nvendor = \"V\"\n\n[notes.128]\nname = \"Impossible\"\ntag = \"kick.1\"\n",
        );
        assert!(result.map.is_none());
        assert!(
            result.issues[0].message.contains("invalid MIDI note key"),
            "{:?}",
            result.issues
        );
    }

    #[test]
    fn an_imported_map_keeps_every_note() {
        let toml_text = map_to_toml_inner(sample()).unwrap();
        let parsed = parse_map_toml_inner(&toml_text).map.unwrap();
        assert_eq!(parsed.notes.len(), 2);
        assert_eq!(parsed.fallback.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn builtin_maps_come_back_as_editable_copies() {
        let gm = builtin_map_inner("gm").unwrap();
        assert_eq!(gm.id, "gm");
        assert_eq!(gm.notes.len(), 47);
        // Whatever the built-in carries (GM documents fallback rules), the
        // copy round-trips through validation and the TOML writer.
        // A full copy of the built-in validates and serializes cleanly.
        let input = MapInput {
            id: gm.id.clone(),
            name: gm.name.clone(),
            vendor: gm.vendor.clone(),
            description: gm.description.clone(),
            notes: gm
                .notes
                .iter()
                .map(|n| MapNoteInput {
                    note: n.note,
                    name: n.name.clone(),
                    tag: n.tag.clone(),
                })
                .collect(),
            fallback: gm.fallback.as_ref().map(|rules| {
                rules
                    .iter()
                    .map(|r| MapFallbackRuleInput {
                        from: r.from.clone(),
                        to: r.to.clone(),
                    })
                    .collect()
            }),
        };
        let toml_text = map_to_toml_inner(input).unwrap();
        assert!(parse_map_toml_inner(&toml_text).issues.is_empty());
    }

    #[test]
    fn builtin_unknown_id_is_an_error() {
        let err = builtin_map_inner("no-such-map").unwrap_err();
        assert!(err.contains("unknown map id"), "{err}");
    }
}

//! Drum map data model (SPEC §3).
//!
//! # TOML layout
//!
//! One TOML file per product, e.g. `data/maps/ggd-invasion.toml`:
//!
//! ```toml
//! id = "ggd-invasion"
//! name = "GetGood Drums Invasion"
//! vendor = "GetGood Drums"
//! description = "Invasion kit, 2019."
//!
//! # Notes are keyed by MIDI note number (0-127) as strings; TOML table keys
//! # are always strings, so `36` is written as `[notes.36]`.
//! [notes.36]
//! name = "Kick In"           # display name shown in the report UI
//! tag = "kick.1"             # semantic articulation tag (ADR-001)
//!
//! # Optional per-map fallback rules (SPEC §3.3). Each rule documents a
//! # collapse: when converting FROM this map and the target map has no exact
//! # match for `from`, the converter tries `to` before falling back to the
//! # generic parent-walk. Rules fire in file order; every firing is reported
//! # as LOSSY.
//! [fallback]
//! [[fallback.rules]]
//! from = "hihat.open.4"
//! to = "hihat.open"
//! ```
//!
//! The `fallback` section is optional; without it the converter only uses
//! the generic parent-walk fallback (see `convert`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::tags::ArticulationTag;

/// One drum map (one product).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumMap {
    /// Stable identifier, e.g. `ggd-invasion`. Also the TOML file stem.
    pub id: String,
    /// Display name, e.g. `GetGood Drums Invasion`.
    pub name: String,
    /// Vendor, e.g. `GetGood Drums`. Used for grouping in the UI only.
    pub vendor: String,
    /// Free-form description shown in the UI. Optional.
    #[serde(default)]
    pub description: Option<String>,
    /// Note entries keyed by MIDI note number (0-127). Sorted ascending.
    #[serde(
        deserialize_with = "parse_note_keys",
        serialize_with = "write_note_keys"
    )]
    pub notes: BTreeMap<u8, NoteEntry>,
    /// Documented per-map fallback rules (SPEC §3.3). Optional.
    #[serde(default)]
    pub fallback: Option<FallbackSection>,
}

/// One note entry in a map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    /// Display name, e.g. `Kick In`.
    pub name: String,
    /// Semantic articulation tag used for conversion matching (ADR-001).
    pub tag: ArticulationTag,
}

/// The `[fallback]` section of a map.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FallbackSection {
    /// Documented collapse rules, checked in file order.
    #[serde(default)]
    pub rules: Vec<FallbackRule>,
}

/// One documented fallback rule: try `to` when `from` has no target match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackRule {
    /// The source articulation that is being collapsed.
    pub from: ArticulationTag,
    /// The less specific articulation to try instead.
    pub to: ArticulationTag,
}

impl DrumMap {
    /// Look up a note by MIDI number.
    pub fn note(&self, midi: u8) -> Option<&NoteEntry> {
        self.notes.get(&midi)
    }

    /// All MIDI numbers used by this map, sorted ascending.
    pub fn used_notes(&self) -> Vec<u8> {
        let mut keys: Vec<u8> = self.notes.keys().copied().collect();
        keys.sort_unstable();
        keys
    }

    /// First note number carrying `tag`, if any.
    pub fn note_for_tag(&self, tag: &ArticulationTag) -> Option<u8> {
        self.notes
            .iter()
            .find(|(_, entry)| &entry.tag == tag)
            .map(|(note, _)| *note)
    }
}

/// TOML keys are strings; parse them into MIDI note numbers.
///
/// Also enforces that each articulation tag is unique within the map: two
/// notes sharing a tag would make `note_for_tag` ambiguous, so the TOML is
/// rejected at parse time, naming the duplicate tag and both note numbers.
fn parse_note_keys<'de, D>(deserializer: D) -> Result<BTreeMap<u8, NoteEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = std::collections::HashMap::<String, NoteEntry>::deserialize(deserializer)?;
    let mut notes: BTreeMap<u8, NoteEntry> = BTreeMap::new();
    for (k, v) in raw {
        let note = k
            .parse::<u8>()
            .ok()
            .filter(|n| *n <= 127)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid MIDI note key {k:?}")))?;
        if let Some((prev, tag)) = notes
            .iter()
            .find(|(_, e)| e.tag == v.tag)
            .map(|(n, e)| (*n, e.tag.to_string()))
        {
            return Err(serde::de::Error::custom(format!(
                "duplicate tag {tag} on notes {prev} and {note}"
            )));
        }
        notes.insert(note, v);
    }
    Ok(notes)
}

fn write_note_keys<S>(notes: &BTreeMap<u8, NoteEntry>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let raw: BTreeMap<String, &NoteEntry> = notes.iter().map(|(k, v)| (k.to_string(), v)).collect();
    raw.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_parses_from_toml() {
        let toml_src = r#"
            id = "gm"
            name = "General MIDI"
            vendor = "MMA"

            [notes.36]
            name = "Bass Drum 1"
            tag = "kick.1"

            [notes.38]
            name = "Acoustic Snare"
            tag = "snare.center"
        "#;
        let map: DrumMap = toml::from_str(toml_src).unwrap();
        assert_eq!(map.id, "gm");
        assert_eq!(map.notes.len(), 2);
        assert_eq!(map.notes[&38].tag.to_string(), "snare.center");
        assert!(map.fallback.is_none());
        // BTreeMap: iteration is sorted ascending.
        assert_eq!(map.used_notes(), vec![36, 38]);
    }

    #[test]
    fn map_rejects_duplicate_tags() {
        let toml_src = r#"
            id = "x"
            name = "X"
            vendor = "V"

            [notes.38]
            name = "Snare"
            tag = "snare.center"

            [notes.40]
            name = "Snare Alt"
            tag = "snare.center"
        "#;
        let err = toml::from_str::<DrumMap>(toml_src).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate tag snare.center"), "{msg}");
        assert!(msg.contains("38"), "{msg}");
        assert!(msg.contains("40"), "{msg}");
    }

    #[test]
    fn map_parses_fallback_rules_and_metadata() {
        let toml_src = r#"
            id = "ggd-invasion"
            name = "GetGood Drums Invasion"
            vendor = "GetGood Drums"
            description = "Invasion kit."

            [notes.36]
            name = "Kick"
            tag = "kick.1"

            [fallback]
            [[fallback.rules]]
            from = "hihat.open.4"
            to = "hihat.open"
        "#;
        let map: DrumMap = toml::from_str(toml_src).unwrap();
        assert_eq!(map.description.as_deref(), Some("Invasion kit."));
        let fb = map.fallback.unwrap();
        assert_eq!(fb.rules.len(), 1);
        assert_eq!(fb.rules[0].from.to_string(), "hihat.open.4");
        assert_eq!(fb.rules[0].to.to_string(), "hihat.open");
    }

    #[test]
    fn map_rejects_bad_note_keys() {
        let toml_src = r#"
            id = "x"
            name = "X"
            vendor = "V"

            [notes.128]
            name = "Impossible"
            tag = "kick.1"
        "#;
        assert!(toml::from_str::<DrumMap>(toml_src).is_err());
    }

    #[test]
    fn helpers() {
        let toml_src = r#"
            id = "x"
            name = "X"
            vendor = "V"

            [notes.36]
            name = "Kick"
            tag = "kick.1"

            [notes.42]
            name = "Closed HH"
            tag = "hihat.closed.edge"
        "#;
        let map: DrumMap = toml::from_str(toml_src).unwrap();
        assert_eq!(map.used_notes(), vec![36, 42]);
        assert_eq!(map.note(36).unwrap().name, "Kick");
        assert_eq!(
            map.note_for_tag(&ArticulationTag::parse("hihat.closed.edge").unwrap()),
            Some(42)
        );
        assert!(
            map.note_for_tag(&ArticulationTag::parse("crash.1").unwrap())
                .is_none()
        );
    }
}

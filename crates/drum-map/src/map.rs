//! Drum map data model (SPEC §3).
//!
//! A drum map is one TOML file per product. Each entry maps a MIDI note
//! number to a display name and a semantic articulation tag.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::tags::ArticulationTag;

/// One drum map (one product).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumMap {
    /// Stable identifier, e.g. `ggd-invasion`.
    pub id: String,
    /// Display name, e.g. `GetGood Drums Invasion`.
    pub name: String,
    /// Vendor, e.g. `GetGood Drums`. Used for grouping in the UI only.
    pub vendor: String,
    /// Note entries keyed by MIDI note number (0-127).
    #[serde(
        deserialize_with = "parse_note_keys",
        serialize_with = "write_note_keys"
    )]
    pub notes: HashMap<u8, NoteEntry>,
}

/// TOML keys are strings; parse them into MIDI note numbers.
fn parse_note_keys<'de, D>(deserializer: D) -> Result<HashMap<u8, NoteEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, NoteEntry>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(k, v)| {
            k.parse::<u8>()
                .ok()
                .filter(|n| *n <= 127)
                .map(|n| (n, v))
                .ok_or_else(|| serde::de::Error::custom(format!("invalid MIDI note key {k:?}")))
        })
        .collect()
}

fn write_note_keys<S>(notes: &HashMap<u8, NoteEntry>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let raw: HashMap<String, &NoteEntry> = notes.iter().map(|(k, v)| (k.to_string(), v)).collect();
    raw.serialize(serializer)
}

/// One note entry in a map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    /// Display name, e.g. `Kick In`.
    pub name: String,
    /// Semantic articulation tag used for conversion matching (ADR-001).
    pub tag: ArticulationTag,
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
    }
}

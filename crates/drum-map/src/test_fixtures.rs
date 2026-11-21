//! Shared `#[cfg(test)]` fixtures for the crate's test modules.

use std::collections::BTreeMap;

use crate::map::{DrumMap, NoteEntry};
use crate::tags::ArticulationTag;

/// Parse a tag, panicking on failure (test helper).
pub fn t(s: &str) -> ArticulationTag {
    ArticulationTag::parse(s).unwrap()
}

/// Build a map from `(name, note, tag)` triples.
pub fn map(id: &str, entries: &[(&str, u8, &str)]) -> DrumMap {
    let notes: BTreeMap<u8, NoteEntry> = entries
        .iter()
        .map(|&(name, note, tag)| {
            (
                note,
                NoteEntry {
                    name: name.to_owned(),
                    tag: t(tag),
                },
            )
        })
        .collect();
    DrumMap {
        id: id.to_owned(),
        name: id.to_owned(),
        vendor: "V".to_owned(),
        description: None,
        notes,
        fallback: None,
    }
}

/// Build a map from bare note numbers, all tagged `kick.1` (detection tests).
pub fn map_notes(id: &str, notes: &[u8]) -> DrumMap {
    let entries: BTreeMap<u8, NoteEntry> = notes
        .iter()
        .map(|&n| {
            (
                n,
                NoteEntry {
                    name: format!("N{n}"),
                    tag: t("kick.1"),
                },
            )
        })
        .collect();
    DrumMap {
        id: id.to_owned(),
        name: id.to_owned(),
        vendor: "V".to_owned(),
        description: None,
        notes: entries,
        fallback: None,
    }
}

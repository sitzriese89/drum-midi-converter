//! The built-in map catalog (`drum_map::catalog`).
//!
//! Each binding is a thin wasm wrapper around a plain-Rust `*_inner`
//! function; the inner functions are what the unit tests exercise (they run
//! natively under `cargo test`).

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// One entry of the built-in catalog.
#[derive(Serialize, Debug)]
pub struct MapInfo {
    /// Stable lowercase kebab-case id: `ggd-invasion`, `gm`.
    pub id: String,
    /// Display name, e.g. `GetGood Drums Invasion`.
    pub name: String,
}

/// One note of a map's vocabulary: MIDI note number and display name.
#[derive(Serialize, Debug)]
pub struct NoteName {
    /// MIDI note number (0-127).
    pub note: u8,
    /// Display name, e.g. `Kick In`.
    pub name: String,
}

/// All built-in maps, in catalog (declaration) order.
#[wasm_bindgen]
pub fn list_maps() -> Result<JsValue, JsError> {
    to_js(&list_maps_inner().map_err(|e| JsError::new(&e))?)
}

/// The full note vocabulary of one map (SPEC §4.2: reassign to any target
/// note), sorted ascending by note number. Throws for an unknown map id.
#[wasm_bindgen]
pub fn map_notes(map_id: &str) -> Result<JsValue, JsError> {
    to_js(&map_notes_inner(map_id).map_err(|e| JsError::new(&e))?)
}

fn list_maps_inner() -> Result<Vec<MapInfo>, String> {
    Ok(drum_map::catalog()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|map| MapInfo {
            id: map.id,
            name: map.name,
        })
        .collect())
}

fn map_notes_inner(map_id: &str) -> Result<Vec<NoteName>, String> {
    let map = drum_map::find_map(map_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown map id \"{map_id}\" in the embedded catalog"))?;
    // BTreeMap: iteration is already sorted ascending by note number.
    Ok(map
        .notes
        .into_iter()
        .map(|(note, entry)| NoteName {
            note,
            name: entry.name,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_maps_shape() {
        let maps = list_maps_inner().unwrap();
        assert_eq!(maps.len(), 7);
        assert_eq!(maps[0].id, "gm");
        assert_eq!(maps[0].name, "General MIDI");
        assert!(maps.iter().all(|m| !m.id.is_empty() && !m.name.is_empty()));
        assert!(maps.iter().any(|m| m.id == "ggd-invasion"));
    }

    #[test]
    fn map_notes_full_vocabulary() {
        let notes = map_notes_inner("gm").unwrap();
        assert_eq!(notes.len(), 47);
        // Sorted ascending, note -> display name.
        assert!(notes.windows(2).all(|w| w[0].note < w[1].note));
        let kick = notes.iter().find(|n| n.note == 36).unwrap();
        assert_eq!(kick.name, "Bass Drum 1");
    }

    #[test]
    fn map_notes_unknown_id_is_an_error() {
        let err = map_notes_inner("no-such-map").unwrap_err();
        assert!(err.contains("unknown map id \"no-such-map\""), "{err}");
    }
}

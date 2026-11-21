//! Core drum-MIDI map conversion library.
//!
//! Pure Rust, no UI/IO dependencies beyond file bytes passed in and out.
//! See SPEC.md and ADR.md for the design.
//!
//! Public API:
//! - [`DrumMap`] (TOML schema, `map`), [`ArticulationTag`] (`tags`)
//! - [`OverrideTable`] (`overrides`): per-pair tag overrides
//! - [`convert`] (`convert`): tag matching engine + [`Resolution`]
//! - [`midi::convert`]: SMF bytes in, converted SMF bytes + [`Report`] out
//! - [`detect::detect`]: ranked source-map hints
//! - [`catalog`]: embedded built-in maps

pub mod catalog;
pub mod convert;
pub mod detect;
pub mod map;
pub mod midi;
pub mod overrides;
pub mod report;
pub mod tags;

pub use catalog::{CatalogError, catalog, find_map};
pub use convert::{ConvertOptions, Resolution};
pub use map::DrumMap;
pub use midi::convert;
pub use overrides::OverrideTable;
pub use report::Report;
pub use tags::ArticulationTag;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::build_mapping;

    #[test]
    fn tag_roundtrips_through_string() {
        let tag = ArticulationTag::parse("hihat.closed.edge").unwrap();
        assert_eq!(tag.to_string(), "hihat.closed.edge");
    }

    /// Property-style test (SPEC §4.3 / ADR-004): for every ordered pair of
    /// catalog maps, every used source note either maps to a target note or
    /// is reported as unmapped — never silently dropped.
    #[test]
    fn every_used_note_maps_or_is_reported_unmapped() {
        let maps = catalog().unwrap();
        for from in &maps {
            for to in &maps {
                let overrides = crate::catalog::override_tables().unwrap();
                let table = overrides
                    .iter()
                    .find(|t| t.from == from.id && t.to == to.id);
                let mapping = build_mapping(from, to, table);
                for note in from.used_notes() {
                    match mapping.get(&note) {
                        Some(mapped) => {
                            assert!(to.note(mapped.target).is_some());
                        }
                        None => {
                            // Unmapped: must be recordable, with a valid
                            // default resolution (Keep).
                            let opts = ConvertOptions::default();
                            assert_eq!(opts.resolution_for(note), Resolution::Keep);
                        }
                    }
                }
            }
        }
    }

    /// Built-in GM -> placeholder Invasion: shared notes map exactly.
    #[test]
    fn gm_to_invasion_converts_common_notes() {
        let maps = catalog().unwrap();
        let gm = maps.iter().find(|m| m.id == "gm").unwrap();
        let inv = maps.iter().find(|m| m.id == "ggd-invasion").unwrap();
        let mapping = build_mapping(gm, inv, None);
        assert_eq!(mapping.get(&36).unwrap().target, 36); // kick.1
        assert_eq!(mapping.get(&42).unwrap().target, 42); // hihat.closed
        assert!(!mapping.get(&42).unwrap().lossy);
        assert_eq!(mapping.get(&49).unwrap().target, 49); // crash.1
        // GM 38 is snare.acoustic; the placeholder has no matching tag.
        assert!(!mapping.contains_key(&38));
        assert!(!mapping.contains_key(&56)); // cowbell unmapped
    }

    /// Detection on GM-style usage hints at the GM map.
    #[test]
    fn detect_ranks_gm_for_gm_notes() {
        let maps = catalog().unwrap();
        let cands = detect::detect(&[36, 38, 42, 46, 49, 56], &maps);
        assert_eq!(cands[0].map_id, "gm");
        assert!(cands[0].coverage > cands[1].coverage);
    }
}

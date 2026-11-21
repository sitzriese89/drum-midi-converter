//! Built-in map catalog (SPEC §3, roadmap M1/Q8).
//!
//! Maps are embedded at build time via `include_str!` from the repo-level
//! `data/maps/` directory. `catalog()` parses every embedded file and never
//! panics: a broken file surfaces as a named error instead of being skipped
//! silently, so data bugs are caught at first use.
//!
//! Adding a map: drop `data/maps/<id>.toml`, add an `include_str!` entry in
//! [`EMBEDDED_MAPS`], and (if the generic fallback is wrong for some pair)
//! a `data/overrides/<from>-to-<to>.toml` file plus an `EMBEDDED_OVERRIDES`
//! entry.

use thiserror::Error;

use crate::map::DrumMap;
use crate::overrides::OverrideTable;

/// All embedded maps as `(file_name, toml_text)` pairs.
pub const EMBEDDED_MAPS: &[(&str, &str)] = &[
    ("gm.toml", include_str!("../../../data/maps/gm.toml")),
    (
        "ggd-invasion.toml",
        include_str!("../../../data/maps/ggd-invasion.toml"),
    ),
    (
        "ggd-modern-massive.toml",
        include_str!("../../../data/maps/ggd-modern-massive.toml"),
    ),
    (
        "ggd-okw-modern-fusion.toml",
        include_str!("../../../data/maps/ggd-okw-modern-fusion.toml"),
    ),
    ("sd3.toml", include_str!("../../../data/maps/sd3.toml")),
    ("ezd3.toml", include_str!("../../../data/maps/ezd3.toml")),
    (
        "rs-drums-fairview.toml",
        include_str!("../../../data/maps/rs-drums-fairview.toml"),
    ),
];

/// All embedded per-pair override tables as `(file_name, toml_text)` pairs.
pub const EMBEDDED_OVERRIDES: &[(&str, &str)] = &[];

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("failed to parse embedded map {file}: {source}")]
    Map {
        file: &'static str,
        source: toml::de::Error,
    },
    #[error("failed to parse embedded override table {file}: {source}")]
    Override {
        file: &'static str,
        source: crate::overrides::OverrideError,
    },
    #[error("duplicate map id {id:?} in the catalog")]
    DuplicateId { id: String },
    #[error("override file {file} does not follow the <from>-to-<to>.toml naming convention")]
    OverrideName { file: &'static str },
}

/// Parse all embedded maps. Returns them in declaration order.
pub fn catalog() -> Result<Vec<DrumMap>, CatalogError> {
    let mut maps = Vec::with_capacity(EMBEDDED_MAPS.len());
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (file, text) in EMBEDDED_MAPS {
        let map: DrumMap =
            toml::from_str(text).map_err(|source| CatalogError::Map { file, source })?;
        if !seen.insert(map.id.clone()) {
            return Err(CatalogError::DuplicateId { id: map.id });
        }
        maps.push(map);
    }
    Ok(maps)
}

/// Look up one map by id from the embedded catalog.
pub fn find_map(id: &str) -> Result<Option<DrumMap>, CatalogError> {
    Ok(catalog()?.into_iter().find(|m| m.id == id))
}

/// Parse all embedded override tables, verifying each belongs to the pair
/// named by its file stem convention `<from>-to-<to>.toml`.
pub fn override_tables() -> Result<Vec<OverrideTable>, CatalogError> {
    let mut tables = Vec::with_capacity(EMBEDDED_OVERRIDES.len());
    for (file, text) in EMBEDDED_OVERRIDES {
        let stem = file.trim_end_matches(".toml");
        let (expected_from, expected_to) = match stem.split_once("-to-") {
            Some((from, to)) => (from.to_owned(), to.to_owned()),
            None => {
                return Err(CatalogError::OverrideName { file });
            }
        };
        let table = OverrideTable::parse_for(text, &expected_from, &expected_to)
            .map_err(|source| CatalogError::Override { file, source })?;
        tables.push(table);
    }
    Ok(tables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_gm_map() {
        let maps = catalog().unwrap();
        assert!(maps.len() >= 2);
        let gm = maps.iter().find(|m| m.id == "gm").unwrap();
        assert_eq!(gm.name, "General MIDI");
        // GM percussion range 35..=81.
        assert!(gm.note(35).is_some());
        assert!(gm.note(81).is_some());
        assert_eq!(gm.used_notes().len(), 47);
    }

    #[test]
    fn catalog_contains_exactly_seven_maps() {
        let maps = catalog().unwrap();
        assert_eq!(maps.len(), 7);
        let ids: Vec<&str> = maps.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "gm",
                "ggd-invasion",
                "ggd-modern-massive",
                "ggd-okw-modern-fusion",
                "sd3",
                "ezd3",
                "rs-drums-fairview",
            ]
        );
        for map in &maps {
            assert!(!map.notes.is_empty(), "map {} has no notes", map.id);
            assert!(map.name.contains(' '), "map {} has a display name", map.id);
            assert!(!map.vendor.is_empty(), "map {} has a vendor", map.id);
        }
        // Spot-check the new full maps.
        let inv = maps.iter().find(|m| m.id == "ggd-invasion").unwrap();
        assert_eq!(inv.notes.len(), 47);
        assert!(inv.note(24).is_some());
    }

    #[test]
    fn find_map_by_id() {
        assert!(find_map("gm").unwrap().is_some());
        assert!(find_map("no-such-map").unwrap().is_none());
    }

    #[test]
    fn override_tables_parse() {
        assert!(override_tables().is_ok());
    }

    #[test]
    fn gm_tags_are_parseable_and_unique_per_family() {
        let gm = find_map("gm").unwrap().unwrap();
        let mut seen = std::collections::HashSet::new();
        for entry in gm.notes.values() {
            assert!(
                seen.insert(entry.tag.clone()),
                "duplicate tag in gm: {entry:?}"
            );
        }
    }
}

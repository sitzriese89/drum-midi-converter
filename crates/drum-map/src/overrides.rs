//! Per-pair override tables (ADR-001, SPEC §3.2).
//!
//! Where the generic tag-matching rule gives the wrong answer for a specific
//! source/target pair, a small curated override table corrects it. Overrides
//! live in `data/overrides/<from>-to-<to>.toml`, one file per directed pair:
//!
//! ```toml
//! # data/overrides/ggd-invasion-to-gm.toml
//! from = "ggd-invasion"     # must match the source map id
//! to = "gm"                 # must match the target map id
//!
//! # Map source articulation tag -> target articulation tag. Applied after
//! # the exact tag match fails and before the documented fallback rules and
//! # the generic parent-walk. Overrides never fire for notes that already
//! # matched exactly.
//! [[rules]]
//! from = "snare.rimshot"
//! to = "snare.sidestick"
//! ```
//!
//! No built-in pair needs an override yet, so `data/overrides/` is empty;
//! `load_override` returns `None` until a file is added (the file is then
//! embedded with `include_str!` in `catalog`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::tags::ArticulationTag;

/// A curated override table for one directed map pair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverrideTable {
    /// Source map id, e.g. `ggd-invasion`.
    pub from: String,
    /// Target map id, e.g. `gm`.
    pub to: String,
    /// Tag -> tag overrides, keyed by source tag.
    #[serde(with = "as_rules")]
    pub rules: HashMap<ArticulationTag, ArticulationTag>,
}

/// TOML serializes rules as an array of `{ from, to }` tables; convert that
/// into a `HashMap`.
mod as_rules {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::tags::ArticulationTag;

    #[derive(Serialize, Deserialize)]
    struct Rule {
        from: ArticulationTag,
        to: ArticulationTag,
    }

    pub fn serialize<S>(
        rules: &HashMap<ArticulationTag, ArticulationTag>,
        s: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut list: Vec<Rule> = rules
            .iter()
            .map(|(from, to)| Rule {
                from: from.clone(),
                to: to.clone(),
            })
            .collect();
        list.sort_by(|a, b| a.from.cmp(&b.from));
        list.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<HashMap<ArticulationTag, ArticulationTag>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let list = Vec::<Rule>::deserialize(d)?;
        let mut map = HashMap::new();
        for Rule { from, to } in list {
            map.insert(from, to);
        }
        Ok(map)
    }
}

#[derive(Debug, Error)]
pub enum OverrideError {
    #[error(
        "override table is for pair {from:?} -> {to:?}, expected {expected_from:?} -> {expected_to:?}"
    )]
    PairMismatch {
        from: String,
        to: String,
        expected_from: String,
        expected_to: String,
    },
    #[error("invalid override TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

impl OverrideTable {
    /// Parse an override table from TOML text.
    pub fn parse(toml_text: &str) -> Result<Self, OverrideError> {
        let table: OverrideTable = toml::from_str(toml_text)?;
        Ok(table)
    }

    /// Parse and verify the table belongs to the given map pair.
    pub fn parse_for(
        toml_text: &str,
        expected_from: &str,
        expected_to: &str,
    ) -> Result<Self, OverrideError> {
        let table = Self::parse(toml_text)?;
        if table.from != expected_from || table.to != expected_to {
            return Err(OverrideError::PairMismatch {
                from: table.from,
                to: table.to,
                expected_from: expected_from.to_owned(),
                expected_to: expected_to.to_owned(),
            });
        }
        Ok(table)
    }

    /// The target tag for a source tag, if overridden.
    pub fn get(&self, from: &ArticulationTag) -> Option<&ArticulationTag> {
        self.rules.get(from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        let src = r#"
            from = "ggd-invasion"
            to = "gm"

            [[rules]]
            from = "snare.rimshot"
            to = "snare.sidestick"

            [[rules]]
            from = "snare.cross-stick"
            to = "snare.sidestick"
        "#;
        let table = OverrideTable::parse(src).unwrap();
        assert_eq!(table.from, "ggd-invasion");
        assert_eq!(table.rules.len(), 2);
        let rs = ArticulationTag::parse("snare.rimshot").unwrap();
        assert_eq!(table.get(&rs).unwrap().to_string(), "snare.sidestick");
        let text = toml::to_string(&table).unwrap();
        let reparsed = OverrideTable::parse(&text).unwrap();
        assert_eq!(table, reparsed);
    }

    #[test]
    fn parse_for_checks_pair() {
        let src = r#"
            from = "a"
            to = "b"
            [[rules]]
            from = "kick.1"
            to = "kick"
        "#;
        assert!(OverrideTable::parse_for(src, "a", "b").is_ok());
        let err = OverrideTable::parse_for(src, "a", "c").unwrap_err();
        assert!(err.to_string().contains("expected \"a\" -> \"c\""));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(OverrideTable::parse("not a table").is_err());
        assert!(
            OverrideTable::parse(
                "from = \"a\"\nto = \"b\"\n[[rules]]\nfrom = \"bad..tag\"\nto = \"kick\""
            )
            .is_err()
        );
    }
}

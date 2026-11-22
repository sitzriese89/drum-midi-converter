//! Semantic articulation tags (ADR-001).
//!
//! A tag is a dotted path like `hihat.closed.edge`: instrument family first,
//! then increasingly specific qualifiers. Conversions match tags exactly;
//! fallback rules walk the path upward (drop the last qualifier) until a
//! match exists in the target map.

use std::fmt;

use thiserror::Error;

/// A semantic articulation tag, e.g. `snare.rimshot` or `hihat.open.3`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct ArticulationTag(Vec<String>);

#[derive(Debug, Error)]
pub enum TagError {
    #[error("empty articulation tag")]
    Empty,
    #[error(
        "invalid segment {segment:?} in tag {tag:?}: segments must be lowercase ascii alphanumerics or hyphens"
    )]
    InvalidSegment { tag: String, segment: String },
}

impl ArticulationTag {
    /// Parse a dotted tag like `hihat.closed.edge`.
    pub fn parse(s: &str) -> Result<Self, TagError> {
        let segments: Vec<String> = s.split('.').map(str::to_owned).collect();
        if segments.is_empty() || segments.iter().any(String::is_empty) {
            return Err(TagError::Empty);
        }
        for seg in &segments {
            let valid = !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !valid {
                return Err(TagError::InvalidSegment {
                    tag: s.to_owned(),
                    segment: seg.clone(),
                });
            }
        }
        Ok(Self(segments))
    }

    /// The tag with its last (most specific) qualifier dropped, or `None`
    /// if the tag is already a bare family name. Used by fallback rules.
    pub fn parent(&self) -> Option<Self> {
        let mut segments = self.0.clone();
        match segments.len() {
            0 | 1 => None,
            _ => {
                segments.pop();
                Some(Self(segments))
            }
        }
    }

    /// Number of tag segments (specificity); `hihat.open.4` has 3.
    pub fn qualifiers(&self) -> usize {
        self.0.len()
    }

    /// The top-level instrument family, e.g. `hihat` for `hihat.closed.edge`.
    pub fn family(&self) -> &str {
        &self.0[0]
    }
}

/// The tag vocabulary's instrument families (docs/tag-taxonomy.md, ADR-001):
/// the drum-kit families the taxonomy documents, plus the GM/latin-percussion
/// families the built-in maps already use. User-authored maps are validated
/// against this list; new concepts extend the list here (and in the doc),
/// they don't invent parallel names.
pub const KNOWN_FAMILIES: &[&str] = &[
    "kick",
    "snare",
    "tom",
    "hihat",
    "ride",
    "crash",
    "china",
    "splash",
    "stack",
    "bell",
    "clap",
    "shaker",
    "percussion",
    "bongo",
    "conga",
    "timbale",
    "agogo",
    "whistle",
    "guiro",
    "woodblock",
    "cuica",
    "triangle",
];

/// True when `family` is part of the documented tag vocabulary.
pub fn known_family(family: &str) -> bool {
    KNOWN_FAMILIES.contains(&family)
}

impl fmt::Display for ArticulationTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("."))
    }
}

impl TryFrom<String> for ArticulationTag {
    type Error = TagError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<ArticulationTag> for String {
    fn from(tag: ArticulationTag) -> Self {
        tag.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_invalid() {
        assert!(ArticulationTag::parse("").is_err());
        assert!(ArticulationTag::parse("hihat..edge").is_err());
        assert!(ArticulationTag::parse("HiHat.closed").is_err());
    }

    #[test]
    fn parent_walks_up() {
        let tag = ArticulationTag::parse("hihat.open.3").unwrap();
        assert_eq!(tag.parent().unwrap().to_string(), "hihat.open");
        assert_eq!(tag.parent().unwrap().parent().unwrap().to_string(), "hihat");
        assert!(tag.parent().unwrap().parent().unwrap().parent().is_none());
    }

    #[test]
    fn family_is_first_segment() {
        let tag = ArticulationTag::parse("snare.rimshot").unwrap();
        assert_eq!(tag.family(), "snare");
    }

    #[test]
    fn known_families_cover_the_taxonomy() {
        assert!(known_family("hihat"));
        assert!(known_family("percussion"));
        assert!(!known_family("HiHat"));
        assert!(!known_family("gong"));
    }

    /// The taxonomy doc and `KNOWN_FAMILIES` cannot drift: every family named
    /// in docs/tag-taxonomy.md (via its backticked tag examples) plus the
    /// GM-only families the doc defers to `gm.toml` must be exactly the known
    /// list. The doc itself is never edited to make this pass.
    #[test]
    fn the_taxonomy_doc_and_known_families_agree() {
        let doc = include_str!("../../../docs/tag-taxonomy.md");
        let (gm_file, gm_toml) = crate::catalog::EMBEDDED_MAPS
            .iter()
            .find(|(file, _)| *file == "gm.toml")
            .expect("gm.toml is embedded");
        assert_eq!(*gm_file, "gm.toml");

        // Families from the doc: first segments of backticked, well-formed
        // multi-segment tag examples (`hihat.open.3.tip`, `china.N.mute`, …).
        // Uppercase `N` placeholders are fine except in the first segment;
        // file names and broken spans are not tags and are skipped.
        let mut families: Vec<String> = doc
            .lines()
            .flat_map(|line| line.split('`').skip(1).step_by(2))
            .filter(|span| {
                let segments: Vec<&str> = span.split('.').collect();
                segments.len() >= 2
                    && !span.ends_with(".toml")
                    && !span.ends_with(".md")
                    && segments[0]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase())
                    && segments.iter().all(|s| {
                        !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                    })
            })
            .map(|span| span.split('.').next().unwrap().to_owned())
            .collect();

        // The GM-only families: first segments of gm.toml's tag values, which
        // the doc defers to ("plus the GM-only names already in gm.toml").
        for line in gm_toml.lines() {
            if let Some(tag) = line
                .trim()
                .strip_prefix("tag = \"")
                .and_then(|t| t.strip_suffix('"'))
            {
                families.push(tag.split('.').next().unwrap().to_owned());
            }
        }

        families.sort();
        families.dedup();
        let mut known: Vec<String> = KNOWN_FAMILIES.iter().map(|s| s.to_string()).collect();
        known.sort();
        assert_eq!(
            families, known,
            "docs/tag-taxonomy.md and KNOWN_FAMILIES drifted"
        );
    }
}

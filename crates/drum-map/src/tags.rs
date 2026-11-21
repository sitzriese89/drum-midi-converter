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
        "invalid segment {segment:?} in tag {tag:?}: segments must be lowercase ascii alphanumerics"
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
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
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

    /// The top-level instrument family, e.g. `hihat` for `hihat.closed.edge`.
    pub fn family(&self) -> &str {
        &self.0[0]
    }
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
}

//! Tag-matching conversion engine (ADR-001, ADR-004, ADR-005).
//!
//! # Mapping resolution order (per source note)
//!
//! 1. **Exact tag match**: the source note's tag exists verbatim in the
//!    target map.
//! 2. **Per-pair override**: the override table maps the source tag to a
//!    different target tag (see [`crate::overrides`]).
//! 3. **Documented fallback rules** of the source map (SPEC §3.3): if the
//!    rule's `to` tag exists in the target map, it fires. LOSSY.
//! 4. **Generic parent-walk**: drop the last tag qualifier and re-check
//!    steps 1-3 against the shorter tag; repeat down to the bare family
//!    name. LOSSY.
//!
//! Unmapped notes pass through unchanged by default ([`Resolution::Keep`],
//! ADR-004) and are recorded in the report.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::map::DrumMap;
use crate::overrides::OverrideTable;

use crate::tags::ArticulationTag;

/// What to do with an unmapped note (ADR-004, SPEC §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Resolution {
    /// Keep the note number unchanged (default).
    #[default]
    Keep,
    /// Drop the note from the output.
    Drop,
    /// Reassign to a fixed target note number.
    Reassign(u8),
}

/// Conversion options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertOptions {
    /// Default resolution for unmapped notes.
    #[serde(default)]
    pub resolution: Resolution,
    /// Per-note resolution overrides, keyed by source note number; takes
    /// precedence over `resolution` for that note.
    #[serde(default)]
    pub per_note: HashMap<u8, Resolution>,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            resolution: Resolution::Keep,
            per_note: HashMap::new(),
        }
    }
}

impl ConvertOptions {
    /// Explicit per-note resolution, `Some` only when the user set an
    /// override for exactly this note (unmapped AND lossy rows, SPEC §4.3).
    /// Unlike [`Self::resolution_for`] this never falls back to the default,
    /// so it can safely take precedence over the tag mapping.
    pub fn explicit_resolution(&self, note: u8) -> Option<Resolution> {
        self.per_note.get(&note).copied()
    }

    /// Effective resolution for one source note.
    pub fn resolution_for(&self, note: u8) -> Resolution {
        self.per_note.get(&note).copied().unwrap_or(self.resolution)
    }
}

/// How a source note number was resolved into a target note number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedNote {
    /// Target MIDI note number.
    pub target: u8,
    /// Target note display name.
    pub target_name: String,
    /// `true` when the match is not exact (fallback rule or parent walk
    /// fired) — reported as LOSSY.
    pub lossy: bool,
}

/// Result of one `resolve` attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Resolve {
    /// Mapped to a target note.
    Mapped(MappedNote),
    /// No mapping entry exists.
    Unmapped,
}

/// Build the note-number -> note-number mapping for a map pair.
///
/// Resolution order per source note: exact tag, per-pair override,
/// documented fallback rules, then the generic parent-walk. Notes whose tag
/// never resolves are [`Resolve::Unmapped`].
pub fn build_mapping(
    from: &DrumMap,
    to: &DrumMap,
    overrides: Option<&OverrideTable>,
) -> HashMap<u8, MappedNote> {
    let mut mapping = HashMap::new();
    for (&note, entry) in &from.notes {
        if let Resolve::Mapped(mapped) = resolve(from, to, overrides, &entry.tag, 0) {
            mapping.insert(note, mapped);
        }
    }
    mapping
}

/// Build a `MappedNote` for a resolved target note (single construction site).
fn mapped_note(to: &DrumMap, target: u8, lossy: bool) -> MappedNote {
    MappedNote {
        target,
        target_name: to.note(target).map(|e| e.name.clone()).unwrap_or_default(),
        lossy,
    }
}

/// Resolve one source articulation tag against the target map.
///
/// `depth` counts parent-walk steps: any match found after at least one
/// fallback firing is lossy (ADR-005).
fn resolve(
    from: &DrumMap,
    to: &DrumMap,
    overrides: Option<&OverrideTable>,
    tag: &ArticulationTag,
    depth: usize,
) -> Resolve {
    let lossy = depth > 0;
    // 1. Exact tag match.
    if let Some(target) = to.note_for_tag(tag) {
        return Resolve::Mapped(mapped_note(to, target, lossy));
    }
    // 2. Per-pair override (tag -> different tag), then re-check exactly.
    if let Some(target) = overrides
        .and_then(|o| o.get(tag))
        .and_then(|t| to.note_for_tag(t))
    {
        return Resolve::Mapped(mapped_note(to, target, lossy));
    }
    // 3. Documented fallback rules of the source map (SPEC §3.3).
    if let Some(fallback) = &from.fallback {
        for rule in &fallback.rules {
            if &rule.from == tag && to.note_for_tag(&rule.to).is_some() {
                return mapped_lossy(to, &rule.to);
            }
        }
    }
    // 4. Generic parent-walk: drop the last qualifier and re-check.
    if let Some(parent) = tag.parent() {
        return resolve(from, to, overrides, &parent, depth + 1);
    }
    Resolve::Unmapped
}

/// Map a lossy tag match to its target note.
fn mapped_lossy(to: &DrumMap, tag: &ArticulationTag) -> Resolve {
    to.note_for_tag(tag)
        .map(|target| Resolve::Mapped(mapped_note(to, target, true)))
        .unwrap_or(Resolve::Unmapped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{map, t};

    #[test]
    fn exact_tag_match_wins() {
        let from = map("a", &[("Snare", 38, "snare.center")]);
        let to = map("b", &[("Center Snare", 40, "snare.center")]);
        let m = build_mapping(&from, &to, None);
        assert_eq!(m[&38].target, 40);
        assert!(!m[&38].lossy);
    }

    #[test]
    fn override_beats_parent_walk() {
        let from = map("a", &[("Rimshot", 38, "snare.rimshot")]);
        let to = map(
            "b",
            &[("Side Stick", 37, "snare"), ("Center", 40, "snare.center")],
        );
        let ovl = OverrideTable::parse_for(
            r#"
            from = "a"
            to = "b"
            [[rules]]
            from = "snare.rimshot"
            to = "snare.center"
        "#,
            "a",
            "b",
        )
        .unwrap();
        let m = build_mapping(&from, &to, Some(&ovl));
        // Without override the parent walk would land on bare `snare` (37).
        assert_eq!(m[&38].target, 40);
        assert!(!m[&38].lossy);
    }

    #[test]
    fn parent_walk_fires_and_is_lossy() {
        let from = map("a", &[("Open HH 4", 46, "hihat.open.4")]);
        let to = map(
            "b",
            &[
                ("Open HH", 46, "hihat.open"),
                ("Closed HH", 42, "hihat.closed"),
            ],
        );
        let m = build_mapping(&from, &to, None);
        assert_eq!(m[&46].target, 46);
        assert!(m[&46].lossy);
    }

    #[test]
    fn parent_walk_walks_to_family() {
        let from = map("a", &[("Rimshot", 38, "snare.rimshot")]);
        let to = map("b", &[("Snare", 40, "snare")]);
        let m = build_mapping(&from, &to, None);
        assert_eq!(m[&38].target, 40);
        assert!(m[&38].lossy);
    }

    #[test]
    fn documented_rule_fires_before_parent_walk() {
        let mut from = map("a", &[("Open HH 4", 46, "hihat.open.4")]);
        from.fallback = Some(crate::map::FallbackSection {
            rules: vec![crate::map::FallbackRule {
                from: t("hihat.open.4"),
                to: t("hihat.closed"),
            }],
        });
        let to = map(
            "b",
            &[
                ("Open HH", 46, "hihat.open"),
                ("Closed HH", 42, "hihat.closed"),
            ],
        );
        let m = build_mapping(&from, &to, None);
        assert_eq!(m[&46].target, 42);
        assert!(m[&46].lossy);
    }

    #[test]
    fn unresolved_notes_are_unmapped() {
        let from = map("a", &[("Cowbell", 56, "misc.cowbell")]);
        let to = map("b", &[("Snare", 40, "snare")]);
        let m = build_mapping(&from, &to, None);
        assert!(!m.contains_key(&56));
    }

    #[test]
    fn resolution_for_respects_per_note() {
        let mut opts = ConvertOptions::default();
        assert_eq!(opts.resolution_for(56), Resolution::Keep);
        opts.resolution = Resolution::Drop;
        opts.per_note.insert(56, Resolution::Reassign(60));
        assert_eq!(opts.resolution_for(56), Resolution::Reassign(60));
        assert_eq!(opts.resolution_for(38), Resolution::Drop);
    }
}

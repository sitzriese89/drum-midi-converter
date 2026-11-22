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

use crate::tags::{ArticulationTag, known_family};

/// Longest accepted map id, in characters.
pub const MAP_ID_MAX_LEN: usize = 64;

/// True when `id` is a valid map id: 1–64 lowercase letters, digits or `-`,
/// starting with a letter or digit. Lowercase only, so ids can't collide on
/// case-insensitive filesystems. This is the single source of the rule: the
/// id is also the file name in the app-data folder (SPEC §3.4), so
/// `DrumMap::build` validates it here and the Tauri storage layer reuses it.
pub fn valid_map_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAP_ID_MAX_LEN
        && id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FallbackSection {
    /// Documented collapse rules, checked in file order.
    #[serde(default)]
    pub rules: Vec<FallbackRule>,
}

/// One documented fallback rule: try `to` when `from` has no target match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// All note entries as `(note number, entry)` pairs, sorted ascending.
    pub fn entries(&self) -> Vec<(u8, &NoteEntry)> {
        self.notes
            .iter()
            .map(|(note, entry)| (*note, entry))
            .collect()
    }
}

/// One note of a map under construction: MIDI number, display name, tag text.
/// The structured counterpart of a `[notes.N]` TOML table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteInput {
    /// MIDI note number (0-127).
    pub note: u8,
    /// Display name, e.g. `Kick In`. Must not be empty.
    pub name: String,
    /// Semantic tag text, parsed with [`ArticulationTag::parse`].
    pub tag: String,
}

/// One fallback rule under construction: both ends as tag text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackRuleInput {
    /// The source articulation that is being collapsed.
    pub from: String,
    /// The less specific articulation to try instead.
    pub to: String,
}

/// A validation problem in a user-authored map, naming the field it concerns:
/// `id`, `name`, `vendor`, `notes.36`, `notes.36.tag`, `fallback.rules[0].from`, …
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapIssue {
    /// The field the problem concerns, as a path into the map.
    pub field: String,
    /// The full problem message.
    pub message: String,
}

impl DrumMap {
    /// Builds a user-authored map from structured data, validating every
    /// field (SPEC §3.4: the same schema as the built-ins). All problems are
    /// collected, not just the first one, so the editor can show them
    /// field-by-field; the map is built only when the list is empty.
    pub fn build(
        id: &str,
        name: &str,
        vendor: &str,
        description: Option<&str>,
        notes: &[NoteInput],
        fallback: &[FallbackRuleInput],
    ) -> Result<DrumMap, Vec<MapIssue>> {
        let mut issues: Vec<MapIssue> = Vec::new();
        let issue = |field: &str, message: String| MapIssue {
            field: field.to_owned(),
            message,
        };
        if !valid_map_id(id) {
            issues.push(issue(
                "id",
                "the id must be 1-64 lowercase letters, digits or '-', starting with a letter \
                 or digit (it is also the file name)"
                    .into(),
            ));
        }
        if name.is_empty() {
            issues.push(issue("name", "the display name must not be empty".into()));
        }
        if vendor.is_empty() {
            issues.push(issue("vendor", "the vendor must not be empty".into()));
        }
        if notes.is_empty() {
            issues.push(issue("notes", "a map needs at least one note".into()));
        }
        let mut by_note: BTreeMap<u8, NoteEntry> = BTreeMap::new();
        let mut tags: Vec<(ArticulationTag, u8)> = Vec::new();
        for note in notes {
            // Issue fields are keyed by MIDI note NUMBER, not row index: the
            // editor shows them on per-note rows, and indices shift whenever
            // a note is added or removed.
            if by_note.contains_key(&note.note) {
                issues.push(issue(
                    &format!("notes.{}.note", note.note),
                    format!("duplicate MIDI note {}", note.note),
                ));
            }
            if note.name.trim().is_empty() {
                issues.push(issue(
                    &format!("notes.{}.name", note.note),
                    "the note name must not be empty".into(),
                ));
            }
            match parse_known_tag(&note.tag) {
                Ok(tag) => {
                    if let Some((prev, _)) = tags.iter().find(|(t, _)| *t == tag) {
                        issues.push(issue(
                            &format!("notes.{}.tag", note.note),
                            format!("duplicate tag {} (already on note {prev})", note.tag),
                        ));
                    } else {
                        tags.push((tag.clone(), note.note));
                    }
                    by_note.insert(
                        note.note,
                        NoteEntry {
                            name: note.name.clone(),
                            tag,
                        },
                    );
                }
                Err(message) => {
                    issues.push(issue(&format!("notes.{}.tag", note.note), message));
                }
            }
        }
        let mut parsed_rules: Vec<FallbackRule> = Vec::new();
        for (index, rule) in fallback.iter().enumerate() {
            let from = parse_known_tag(&rule.from);
            let to = parse_known_tag(&rule.to);
            for (end, result) in [("from", &from), ("to", &to)] {
                if let Err(message) = result {
                    issues.push(issue(
                        &format!("fallback.rules[{index}].{end}"),
                        message.clone(),
                    ));
                }
            }
            match (&from, &to) {
                (Ok(from), Ok(to)) if from == to => issues.push(issue(
                    &format!("fallback.rules[{index}]"),
                    "a fallback rule must collapse to a LESS specific tag".into(),
                )),
                (Ok(from), Ok(to)) => parsed_rules.push(FallbackRule {
                    from: from.clone(),
                    to: to.clone(),
                }),
                _ => {}
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        Ok(DrumMap {
            id: id.to_owned(),
            name: name.to_owned(),
            vendor: vendor.to_owned(),
            description: description
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(str::to_owned),
            notes: by_note,
            fallback: (!fallback.is_empty()).then_some(FallbackSection {
                rules: parsed_rules,
            }),
        })
    }

    /// Serializes the map back to TOML with the same layout as the built-in
    /// `data/maps/*.toml` files, so a saved user map round-trips through
    /// [`DrumMap`]'s TOML parser unchanged.
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("id = {}\n", toml_string(&self.id)));
        out.push_str(&format!("name = {}\n", toml_string(&self.name)));
        out.push_str(&format!("vendor = {}\n", toml_string(&self.vendor)));
        if let Some(description) = &self.description {
            out.push_str(&format!("description = {}\n", toml_string(description)));
        }
        for (note, entry) in &self.notes {
            out.push_str(&format!("\n[notes.{note}]\n"));
            out.push_str(&format!("name = {}\n", toml_string(&entry.name)));
            out.push_str(&format!("tag = {}\n", toml_string(&entry.tag.to_string())));
        }
        if let Some(fallback) = &self.fallback {
            out.push_str("\n[fallback]\n");
            for rule in &fallback.rules {
                out.push_str(&format!(
                    "\n[[fallback.rules]]\nfrom = {}\nto = {}\n",
                    toml_string(&rule.from.to_string()),
                    toml_string(&rule.to.to_string())
                ));
            }
        }
        out
    }
}

/// Parses a tag for validation: a parse error and a family outside the
/// ADR-001 vocabulary both come back as the message for the field's issue.
fn parse_known_tag(tag: &str) -> Result<ArticulationTag, String> {
    match ArticulationTag::parse(tag) {
        Ok(tag) if known_family(tag.family()) => Ok(tag),
        Ok(tag) => Err(format!(
            "unknown tag family {:?}: use the ADR-001 vocabulary (docs/tag-taxonomy.md)",
            tag.family()
        )),
        Err(e) => Err(e.to_string()),
    }
}

/// A TOML basic string, with the escapes the TOML format requires.
fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Control characters as unicode escapes; everything else as-is.
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
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

    /// `build` accepts a valid map: all fields land where the editor put them.
    #[test]
    fn build_accepts_a_valid_map() {
        let map = DrumMap::build(
            "my-kit",
            "My Kit",
            "Me",
            Some("Home kit."),
            &[
                NoteInput {
                    note: 36,
                    name: "Kick".into(),
                    tag: "kick.1".into(),
                },
                NoteInput {
                    note: 38,
                    name: "Snare".into(),
                    tag: "snare.center".into(),
                },
            ],
            &[FallbackRuleInput {
                from: "hihat.open.2".into(),
                to: "hihat.open".into(),
            }],
        )
        .unwrap();
        assert_eq!(map.id, "my-kit");
        assert_eq!(map.description.as_deref(), Some("Home kit."));
        assert_eq!(map.used_notes(), vec![36, 38]);
        assert_eq!(map.fallback.unwrap().rules.len(), 1);
    }

    /// `build` collects every problem with its field, not just the first.
    #[test]
    fn build_reports_all_issues_field_by_field() {
        let issues = DrumMap::build(
            "",
            "",
            "",
            None,
            &[
                NoteInput {
                    note: 38,
                    name: "".into(),
                    tag: "Snare!".into(),
                },
                NoteInput {
                    note: 36,
                    name: "Kick".into(),
                    tag: "kick.1".into(),
                },
                NoteInput {
                    note: 36,
                    name: "Kick 2".into(),
                    tag: "gong.1".into(),
                },
            ],
            &[],
        )
        .unwrap_err();
        let fields: Vec<&str> = issues.iter().map(|i| i.field.as_str()).collect();
        assert!(fields.contains(&"id"), "{issues:?}");
        assert!(fields.contains(&"name"), "{issues:?}");
        assert!(fields.contains(&"vendor"), "{issues:?}");
        // Per-note issues are keyed by MIDI note NUMBER, not row index: the
        // editor shows them on the matching note row.
        assert!(fields.contains(&"notes.38.name"), "{issues:?}");
        assert!(fields.contains(&"notes.38.tag"), "{issues:?}");
        // The gong family is outside the taxonomy, and note 36 is duplicated.
        assert!(fields.contains(&"notes.36.tag"), "{issues:?}");
        assert!(fields.contains(&"notes.36.note"), "{issues:?}");
        // The taxonomy check names the vocabulary doc.
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("docs/tag-taxonomy.md"))
        );
    }

    /// The map-id rule is one shared function (the Tauri storage layer
    /// reuses it, the id is also the file name).
    #[test]
    fn valid_map_id_follows_the_shared_rule() {
        for id in ["gm", "my-kit", "kit2", &"a".repeat(MAP_ID_MAX_LEN)] {
            assert!(valid_map_id(id), "{id:?} should be valid");
        }
        for id in ["", "Upper", "with space", "with.dot", "a/b", "-lead"] {
            assert!(!valid_map_id(id), "{id:?} should be invalid");
        }
        assert!(!valid_map_id(&"a".repeat(MAP_ID_MAX_LEN + 1)));
    }

    /// A `from == to` fallback rule is a no-op and is rejected.
    #[test]
    fn build_rejects_a_noop_fallback_rule() {
        let issues = DrumMap::build(
            "x",
            "X",
            "V",
            None,
            &[NoteInput {
                note: 36,
                name: "Kick".into(),
                tag: "kick.1".into(),
            }],
            &[FallbackRuleInput {
                from: "hihat.open".into(),
                to: "hihat.open".into(),
            }],
        )
        .unwrap_err();
        assert!(
            issues[0].field.starts_with("fallback.rules[0]"),
            "{issues:?}"
        );
    }

    /// `to_toml` round-trips through the TOML parser: what the editor saves
    /// is what a later load reads back (SPEC §3.4: same schema as built-ins).
    #[test]
    fn to_toml_round_trips() {
        let map = DrumMap::build(
            "my-kit",
            "My \"Kit\"",
            "Me",
            Some("Home kit."),
            &[
                NoteInput {
                    note: 36,
                    name: "Kick".into(),
                    tag: "kick.1".into(),
                },
                NoteInput {
                    note: 42,
                    name: "Closed HH".into(),
                    tag: "hihat.closed.edge".into(),
                },
            ],
            &[FallbackRuleInput {
                from: "hihat.open.2".into(),
                to: "hihat.open".into(),
            }],
        )
        .unwrap();
        let text = map.to_toml();
        let parsed: DrumMap = toml::from_str(&text).unwrap();
        assert_eq!(parsed.id, map.id);
        assert_eq!(parsed.name, map.name);
        assert_eq!(parsed.description, map.description);
        assert_eq!(parsed.used_notes(), vec![36, 42]);
        assert_eq!(parsed.note(36).unwrap().tag, map.note(36).unwrap().tag);
        assert_eq!(parsed.fallback, map.fallback);
        // Empty description is omitted, like in the built-in files.
        let bare = DrumMap::build(
            "y",
            "Y",
            "V",
            None,
            &[NoteInput {
                note: 36,
                name: "Kick".into(),
                tag: "kick.1".into(),
            }],
            &[],
        )
        .unwrap();
        assert!(!bare.to_toml().contains("description"));
        assert!(!bare.to_toml().contains("fallback"));
    }

    /// Every built-in map serializes back to exactly itself (the writer and
    /// the parser agree on the schema).
    #[test]
    fn every_builtin_round_trips_through_to_toml() {
        for map in crate::catalog().unwrap() {
            let text = map.to_toml();
            let parsed: DrumMap = toml::from_str(&text).unwrap();
            assert_eq!(parsed.id, map.id);
            assert_eq!(parsed.used_notes(), map.used_notes());
            for (note, entry) in map.entries() {
                assert_eq!(parsed.note(note).unwrap().name, entry.name);
                assert_eq!(parsed.note(note).unwrap().tag, entry.tag);
            }
            assert_eq!(parsed.fallback, map.fallback);
        }
    }

    /// `entries` lists every note in ascending order.
    #[test]
    fn entries_are_sorted_ascending() {
        let map = DrumMap::build(
            "x",
            "X",
            "V",
            None,
            &[
                NoteInput {
                    note: 42,
                    name: "Hat".into(),
                    tag: "hihat.closed.edge".into(),
                },
                NoteInput {
                    note: 36,
                    name: "Kick".into(),
                    tag: "kick.1".into(),
                },
            ],
            &[],
        )
        .unwrap();
        let entries = map.entries();
        assert_eq!(
            entries.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            vec![36, 42]
        );
        assert_eq!(entries[0].1.name, "Kick");
    }
}

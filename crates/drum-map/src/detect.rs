//! Detection heuristic (SPEC §4.1, ADR-003).
//!
//! Detection is a *hint*, never a decision: the function returns ranked
//! candidates with a confidence score so the UI can pre-select one and warn
//! on a mismatch. Scoring: a candidate map earns the fraction of used notes
//! it contains, weighted by how specific each matched note's tag is (a
//! multi-qualifier tag like `hihat.open.4` proves more than a bare family
//! name like `kick`).

use crate::map::DrumMap;

/// One ranked detection candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// Candidate map id.
    pub map_id: String,
    /// Confidence score in `0.0..=1.0`.
    pub confidence: f64,
    /// Fraction of the used notes present in the candidate map.
    pub coverage: f64,
    /// Sum of tag qualifiers in the candidate map (tie-break evidence).
    pub specificity: usize,
}

/// Weight of a tag match by specificity: number of tag qualifiers.
fn tag_weight(qualifiers: usize) -> f64 {
    // 1 qualifier = 1.0, 2 = 1.25, 3 = 1.5, ... — diminishing returns.
    1.0 + 0.25 * (qualifiers.saturating_sub(1)) as f64
}

/// Rank all catalog maps against the used note numbers.
///
/// `used` is the set of distinct MIDI note numbers actually played in the
/// file. Returns candidates sorted by descending confidence; ties break by
/// map id for determinism.
pub fn detect(used: &[u8], catalog: &[DrumMap]) -> Vec<Candidate> {
    if used.is_empty() || catalog.is_empty() {
        return Vec::new();
    }
    let mut used: Vec<u8> = used.to_vec();
    used.sort_unstable();
    used.dedup();

    let mut candidates: Vec<Candidate> = catalog
        .iter()
        .map(|map| {
            let mut weighted_hits = 0.0f64;
            let mut hits = 0usize;
            for &note in &used {
                if let Some(entry) = map.note(note) {
                    hits += 1;
                    weighted_hits += tag_weight(entry.tag.qualifiers());
                }
            }
            let coverage = hits as f64 / used.len() as f64;
            let confidence = (weighted_hits / used.len() as f64).min(1.0);
            let specificity = map
                .notes
                .values()
                .map(|e| e.tag.qualifiers())
                .sum::<usize>();
            Candidate {
                map_id: map.id.clone(),
                confidence,
                coverage,
                specificity,
            }
        })
        .collect();
    // Confidence is capped at 1.0, so ties are broken by tag specificity
    // (more specific tags are stronger evidence), then by map id for
    // determinism.
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.specificity.cmp(&a.specificity))
            .then_with(|| a.map_id.cmp(&b.map_id))
    });
    candidates
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::map::NoteEntry;
    use crate::tags::ArticulationTag;

    fn t(s: &str) -> ArticulationTag {
        ArticulationTag::parse(s).unwrap()
    }

    fn map(id: &str, entries: &[u8]) -> DrumMap {
        let mut notes = HashMap::new();
        for &note in entries {
            notes.insert(
                note,
                NoteEntry {
                    name: format!("N{note}"),
                    tag: t("kick.1"),
                },
            );
        }
        DrumMap {
            id: id.to_owned(),
            name: id.to_owned(),
            vendor: "V".to_owned(),
            description: None,
            notes,
            fallback: None,
        }
    }

    #[test]
    fn perfect_match_scores_one() {
        let catalog = vec![map("a", &[36, 38, 42]), map("b", &[36, 38])];
        let cands = detect(&[36, 38, 42], &catalog);
        assert_eq!(cands[0].map_id, "a");
        assert!((cands[0].confidence - 1.0).abs() < 1e-9);
        assert_eq!(cands[1].map_id, "b");
    }

    #[test]
    fn partial_match_scores_fraction() {
        let catalog = vec![map("a", &[36, 38])];
        let cands = detect(&[36, 38, 42], &catalog);
        assert_eq!(cands[0].map_id, "a");
        assert!((cands[0].coverage - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn specific_tags_outscore_generic_ones() {
        let mut specific = map("specific", &[38]);
        specific.notes.get_mut(&38).unwrap().tag = t("snare.rimshot");
        let mut generic = map("generic", &[38]);
        generic.notes.get_mut(&38).unwrap().tag = t("kick");
        let catalog = vec![specific, generic];
        // Both cover 38 with capped confidence 1.0; the specificity
        // tie-break ranks the specific tag first.
        let cands = detect(&[38], &catalog);
        assert_eq!(cands[0].map_id, "specific");
        assert!(cands[0].specificity > cands[1].specificity);
    }

    #[test]
    fn no_overlap_scores_zero_but_still_listed() {
        let catalog = vec![map("a", &[60])];
        let cands = detect(&[36], &catalog);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].confidence, 0.0);
    }

    #[test]
    fn empty_inputs_return_no_candidates() {
        assert!(detect(&[], &[map("a", &[36])]).is_empty());
        assert!(detect(&[36], &[]).is_empty());
    }

    #[test]
    fn ties_break_by_map_id() {
        let catalog = vec![map("zz", &[36, 38]), map("aa", &[36, 38])];
        let cands = detect(&[36, 38], &catalog);
        assert_eq!(cands[0].map_id, "aa");
        assert_eq!(cands[1].map_id, "zz");
    }
}

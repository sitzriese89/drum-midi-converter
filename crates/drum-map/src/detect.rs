//! Detection heuristic (SPEC §4.1, ADR-003).
//!
//! Detection is a *hint*, never a decision: the function returns ranked
//! candidates with a confidence score so the UI can pre-select one and warn
//! on a mismatch. Scoring: confidence is the plain coverage fraction (the
//! share of used notes the candidate map contains), so it honestly means
//! "how much of the file's vocabulary this map explains". Tag specificity
//! is used only as a tie-break when maps cover the same notes.

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
            let hits = used
                .iter()
                .filter(|&&note| map.note(note).is_some())
                .count();
            // Confidence is the plain coverage fraction: a map missing a
            // third of the used notes must never score 1.0.
            let coverage = hits as f64 / used.len() as f64;
            let confidence = coverage;
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
    // Confidence is the coverage fraction, so ties are broken by tag
    // specificity (more specific tags are stronger evidence), then by map
    // id for determinism.
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
    use super::*;
    use crate::test_fixtures::{map_notes as map, t};

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
        // Confidence equals the coverage fraction.
        assert!((cands[0].confidence - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn specific_tags_outscore_generic_ones() {
        let mut specific = map("specific", &[38]);
        specific.notes.get_mut(&38).unwrap().tag = t("snare.rimshot");
        let mut generic = map("generic", &[38]);
        generic.notes.get_mut(&38).unwrap().tag = t("kick");
        let catalog = vec![specific, generic];
        // Both cover 38 with confidence 1.0; the specificity tie-break
        // ranks the specific tag first.
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

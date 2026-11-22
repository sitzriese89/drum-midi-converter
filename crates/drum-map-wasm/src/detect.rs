//! Detection hint bindings (`drum_map::detect`, `drum_map::midi::used_drum_notes`).
//!
//! Detection is a hint, never a decision (ADR-003): the UI pre-selects a
//! source map from these candidates and warns on a mismatch, but never
//! silently converts.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::to_js;

/// One ranked detection candidate.
#[derive(Serialize)]
pub struct Candidate {
    /// Candidate map id.
    pub map_id: String,
    /// Confidence in `0.0..=1.0` (the plain coverage fraction).
    pub confidence: f64,
}

/// Rank all catalog maps against the drum notes used in `midi_bytes`.
///
/// Returns candidates sorted by descending confidence; empty when the file
/// has no drum notes. Throws a JS `Error` for invalid SMF data.
#[wasm_bindgen]
pub fn detect_hints(midi_bytes: &[u8]) -> Result<JsValue, JsError> {
    to_js(&detect_hints_inner(midi_bytes).map_err(js_error)?)
}

/// How well one candidate map covers the file's drum notes: the fraction of
/// used notes the map contains, in `0.0..=1.0`. `None` when the file has no
/// drum notes or the id is unknown — the UI shows no mismatch warning for
/// either (ADR-003: the user decides).
#[wasm_bindgen]
pub fn source_coverage(midi_bytes: &[u8], map_id: &str) -> Result<Option<f64>, JsError> {
    source_coverage_inner(midi_bytes, map_id).map_err(js_error)
}

fn detect_hints_inner(midi_bytes: &[u8]) -> Result<Vec<Candidate>, String> {
    let used = drum_map::midi::used_drum_notes(midi_bytes).map_err(|e| e.to_string())?;
    let catalog = drum_map::catalog().map_err(|e| e.to_string())?;
    // The core computes the coverage fraction; keep it that way instead of
    // re-deriving it here.
    Ok(drum_map::detect::detect(&used, &catalog)
        .iter()
        .map(|c| Candidate {
            map_id: c.map_id.clone(),
            confidence: c.confidence,
        })
        .collect())
}

fn source_coverage_inner(midi_bytes: &[u8], map_id: &str) -> Result<Option<f64>, String> {
    let used = drum_map::midi::used_drum_notes(midi_bytes).map_err(|e| e.to_string())?;
    if used.is_empty() {
        return Ok(None);
    }
    let Some(map) = drum_map::find_map(map_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    // Derive the fraction from the core's detector instead of duplicating
    // its filter/count logic: detect over a one-map "catalog" and read the
    // coverage of the matching candidate.
    Ok(drum_map::detect::detect(&used, &[map])
        .into_iter()
        .next()
        .map(|c| c.coverage))
}

/// Convert a core error into a JS `Error`.
fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::drum_smf;

    #[test]
    fn detect_hints_rank_the_catalog() {
        let hints = detect_hints_inner(&drum_smf()).unwrap();
        assert_eq!(hints.len(), 7);
        // Sorted by descending confidence.
        assert!(hints.windows(2).all(|w| w[0].confidence >= w[1].confidence));
        // GM contains both 36 and 56: a perfect-coverage candidate exists.
        assert!(
            hints
                .iter()
                .any(|h| h.map_id == "gm" && h.confidence == 1.0)
        );
    }

    #[test]
    fn detect_hints_rejects_invalid_smf() {
        assert!(detect_hints_inner(&[0u8; 4]).is_err());
    }

    #[test]
    fn source_coverage_matches_the_core_detector() {
        // GM contains both used notes (36 kick, 56 cowbell).
        assert_eq!(source_coverage_inner(&drum_smf(), "gm").unwrap(), Some(1.0));
        let coverage = source_coverage_inner(&drum_smf(), "ggd-invasion").unwrap();
        assert!(coverage.is_some_and(|c| (0.0..=1.0).contains(&c)));
    }

    #[test]
    fn source_coverage_none_for_unknown_id_or_silent_file() {
        assert_eq!(
            source_coverage_inner(&drum_smf(), "no-such-map").unwrap(),
            None
        );
        // A file with no drum notes: no coverage, no warning (ADR-003).
        let empty = crate::test_support::no_drum_smf();
        assert_eq!(source_coverage_inner(&empty, "gm").unwrap(), None);
    }
}

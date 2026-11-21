//! Conversion report data model (SPEC §4.3).
//!
//! One row per used note, table-first. The UI renders this as a table with
//! LOSSY badges and per-note override dropdowns; the core only produces the
//! data.

use serde::{Deserialize, Serialize};

/// One row of the conversion report: one used source note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRow {
    /// Source MIDI note number.
    pub source_note: u8,
    /// Source note display name, e.g. `Kick In`.
    pub source_name: String,
    /// Number of note-on hits (velocity > 0) for this note in the file.
    pub hit_count: u32,
    /// Mapped target MIDI note number, or `None` when unmapped.
    pub target_note: Option<u8>,
    /// Target note display name, or `None` when unmapped.
    pub target_name: Option<String>,
    /// `true` when a fallback rule fired for this note (LOSSY badge).
    pub lossy: bool,
    /// `true` when the note had no mapping entry (kept/dropped/reassigned
    /// per the resolution option, ADR-004).
    pub unmapped: bool,
}

/// Full conversion report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Report {
    /// One row per used source note, sorted by source note number.
    pub rows: Vec<NoteRow>,
}

impl Report {
    /// Rows with LOSSY badges.
    pub fn lossy(&self) -> impl Iterator<Item = &NoteRow> {
        self.rows.iter().filter(|row| row.lossy)
    }

    /// Rows for unmapped notes.
    pub fn unmapped(&self) -> impl Iterator<Item = &NoteRow> {
        self.rows.iter().filter(|row| row.unmapped)
    }
}

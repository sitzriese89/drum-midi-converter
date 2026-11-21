//! Core drum-MIDI map conversion library.
//!
//! Pure Rust, no UI/IO dependencies beyond file bytes passed in and out.
//! See SPEC.md and ADR.md for the design.

pub mod map;
pub mod tags;

pub use map::DrumMap;
pub use tags::ArticulationTag;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_roundtrips_through_string() {
        let tag = ArticulationTag::parse("hihat.closed.edge").unwrap();
        assert_eq!(tag.to_string(), "hihat.closed.edge");
    }
}

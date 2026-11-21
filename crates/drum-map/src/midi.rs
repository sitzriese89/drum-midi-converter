//! MIDI I/O via `midly` (ADR-007).
//!
//! Parse SMF bytes, rewrite note events on drum tracks, preserve everything
//! else: format type, timing (division), tempo/time-sig/meta events,
//! markers, SysEx, non-drum tracks, and event order within each track.
//!
//! A track is a *drum track* when it contains note events and all of them
//! are on channel 10 (index 9, `DRUM_CHANNEL`) — the GM percussion channel.
//! A note-on with velocity 0 is treated as note-off (SMF convention) for
//! mapping purposes; when serialized it keeps its original note-on status so
//! the byte shape of the file is preserved.
//!
//! Unmapped notes follow the [`Resolution`] policy (ADR-004): by default
//! they pass through unchanged and are listed in the report.

use std::collections::HashMap;

use midly::{
    MidiMessage, Smf, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u28},
};
use thiserror::Error;

use crate::convert::{ConvertOptions, MappedNote, Resolution, build_mapping};
use crate::map::DrumMap;
use crate::overrides::OverrideTable;
use crate::report::{NoteRow, Report};

/// GM percussion channel, index 9 (channel 10 in 1-based convention).
pub const DRUM_CHANNEL: u4 = u4::new(9);

#[derive(Debug, Error)]
pub enum MidiError {
    #[error("invalid SMF data: {0}")]
    Parse(#[from] midly::Error),
    #[error("failed to serialize SMF: {0}")]
    Write(#[from] std::io::Error),
}

/// Everything needed to rewrite one drum track: the tag-derived note mapping
/// plus the options governing unmapped notes.
#[derive(Debug, Clone)]
pub struct NoteResolution {
    /// Tag-derived note-number mapping (exact + override + fallbacks).
    pub mapping: HashMap<u8, MappedNote>,
    /// Options for unmapped notes.
    pub options: ConvertOptions,
}

impl NoteResolution {
    /// Target note for a mapped source note.
    pub fn target(&self, note: u8) -> Option<u8> {
        self.mapping.get(&note).map(|m| m.target)
    }

    /// Whether the source note has no mapping entry.
    pub fn is_unmapped(&self, note: u8) -> bool {
        !self.mapping.contains_key(&note)
    }

    /// Effective resolution for an unmapped source note.
    pub fn resolution_for(&self, note: u8) -> Resolution {
        self.options.resolution_for(note)
    }

    /// Output note number for a source note, or `None` to drop it.
    fn output(&self, note: u8) -> Option<u8> {
        match self.mapping.get(&note) {
            Some(m) => Some(m.target),
            None => match self.resolution_for(note) {
                Resolution::Keep => Some(note),
                Resolution::Drop => None,
                Resolution::Reassign(n) => Some(n),
            },
        }
    }
}

/// True when the track contains note events and every one is on the drum
/// channel. Tracks with no note events at all are never drum tracks.
pub fn is_drum_track(track: &Track<'_>) -> bool {
    let mut has_note = false;
    for event in track {
        if let TrackEventKind::Midi { channel, message } = event.kind {
            if matches!(
                message,
                MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }
            ) && channel != DRUM_CHANNEL
            {
                return false;
            }
            if matches!(
                message,
                MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }
            ) {
                has_note = true;
            }
        }
    }
    has_note
}

/// Rewrite all note events on drum tracks according to `resolution`.
///
/// Non-drum tracks and non-note events pass through untouched; event order
/// and delta timing are preserved (dropped events donate their delta to the
/// following event; a dropped tail keeps the track length via a trailing
/// EndOfTrack).
pub fn rewrite_notes(
    smf_bytes: &[u8],
    resolution: &NoteResolution,
) -> Result<(Vec<u8>, Report), MidiError> {
    let smf = Smf::parse(smf_bytes)?;
    let mut report = Report::default();
    let mut tracks: Vec<Track> = Vec::with_capacity(smf.tracks.len());
    for track in &smf.tracks {
        let drum = is_drum_track(track);
        let mut new_track: Track = Vec::with_capacity(track.len());
        let mut pending_delta: u28 = u28::new(0);
        for event in track {
            let rewritten = if drum {
                match event.kind {
                    TrackEventKind::Midi { channel, message } if channel == DRUM_CHANNEL => {
                        match message {
                            MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                                let src = key.as_int();
                                match report.rows.iter_mut().find(|r| r.source_note == src) {
                                    Some(row) => row.hit_count += 1,
                                    None => report.rows.push(NoteRow {
                                        source_note: src,
                                        source_name: String::new(),
                                        hit_count: 1,
                                        target_note: None,
                                        target_name: None,
                                        lossy: false,
                                        unmapped: resolution.is_unmapped(src),
                                    }),
                                }
                                resolution.output(src).map(|target| TrackEventKind::Midi {
                                    channel,
                                    message: MidiMessage::NoteOn {
                                        key: u7::new(target),
                                        vel,
                                    },
                                })
                            }
                            MidiMessage::NoteOff { key, vel } => {
                                let src = key.as_int();
                                resolution.output(src).map(|target| TrackEventKind::Midi {
                                    channel,
                                    message: MidiMessage::NoteOff {
                                        key: u7::new(target),
                                        vel,
                                    },
                                })
                            }
                            // Velocity-0 note-on = note-off (SMF convention).
                            // Kept as NoteOn so the byte shape is preserved.
                            MidiMessage::NoteOn { key, vel } => {
                                let src = key.as_int();
                                resolution.output(src).map(|target| TrackEventKind::Midi {
                                    channel,
                                    message: MidiMessage::NoteOn {
                                        key: u7::new(target),
                                        vel,
                                    },
                                })
                            }
                            other => Some(TrackEventKind::Midi {
                                channel,
                                message: other,
                            }),
                        }
                    }
                    _ => Some(event.kind),
                }
            } else {
                Some(event.kind)
            };
            match rewritten {
                Some(kind) => {
                    new_track.push(TrackEvent {
                        delta: event.delta + pending_delta,
                        kind,
                    });
                    pending_delta = u28::new(0);
                }
                // Dropped event: hand its delta to the next surviving event.
                None => pending_delta += event.delta,
            }
        }
        if pending_delta > u28::new(0) {
            new_track.push(TrackEvent {
                delta: pending_delta,
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            });
        }
        tracks.push(new_track);
    }

    let out_smf = Smf {
        header: smf.header,
        tracks,
    };
    let mut out = Vec::new();
    out_smf.write_std(&mut out)?;
    Ok((out, report))
}

/// High-level conversion entry point (SPEC §4.2).
///
/// Takes SMF bytes, the source map, the target map, an optional per-pair
/// override table, and conversion options; returns the converted SMF bytes
/// plus the report.
pub fn convert(
    smf_bytes: &[u8],
    from: &DrumMap,
    to: &DrumMap,
    overrides: Option<&OverrideTable>,
    options: &ConvertOptions,
) -> Result<(Vec<u8>, Report), MidiError> {
    let mapping = build_mapping(from, to, overrides);
    let resolution = NoteResolution {
        mapping,
        options: options.clone(),
    };
    let (bytes, mut report) = rewrite_notes(smf_bytes, &resolution)?;
    for row in &mut report.rows {
        if let Some(mapped) = resolution.mapping.get(&row.source_note) {
            row.target_note = Some(mapped.target);
            row.target_name = Some(mapped.target_name.clone());
            row.lossy = mapped.lossy;
            row.unmapped = false;
        } else {
            match resolution.resolution_for(row.source_note) {
                Resolution::Keep => {
                    row.target_note = Some(row.source_note);
                    row.target_name = to.note(row.source_note).map(|e| e.name.clone());
                }
                Resolution::Drop => {
                    row.target_note = None;
                    row.target_name = None;
                }
                Resolution::Reassign(n) => {
                    row.target_note = Some(n);
                    row.target_name = to.note(n).map(|e| e.name.clone());
                }
            }
        }
        row.source_name = from
            .note(row.source_note)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| format!("Unknown ({})", row.source_note));
    }
    report.rows.sort_by_key(|r| r.source_note);
    Ok((bytes, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::NoteEntry;
    use crate::tags::ArticulationTag;
    use midly::Timing;
    use midly::num::{u15, u24};

    fn t(s: &str) -> ArticulationTag {
        ArticulationTag::parse(s).unwrap()
    }

    fn map(id: &str, entries: &[(&str, u8, &str)]) -> DrumMap {
        let mut notes = HashMap::new();
        for (name, note, tag) in entries {
            notes.insert(
                *note,
                NoteEntry {
                    name: (*name).to_owned(),
                    tag: t(tag),
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

    /// Format-1 SMF: track 0 = tempo/time-sig/marker; track 1 = drums on
    /// channel 10 (incl. a velocity-0 note-on); track 2 = melody on ch 1.
    fn build_smf() -> Vec<u8> {
        let header = midly::Header::new(midly::Format::Parallel, Timing::Metrical(u15::new(480)));
        let track0: Track = vec![
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(midly::MetaMessage::Tempo(u24::new(500_000))),
            },
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(midly::MetaMessage::TimeSignature(4, 2, 24, 8)),
            },
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(midly::MetaMessage::Marker(b"verse")),
            },
            TrackEvent {
                delta: u28::new(1920),
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            },
        ];

        let track1: Track = vec![
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Midi {
                    channel: DRUM_CHANNEL,
                    message: MidiMessage::NoteOn {
                        key: u7::new(36),
                        vel: u7::new(100),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(240),
                kind: TrackEventKind::Midi {
                    channel: DRUM_CHANNEL,
                    message: MidiMessage::NoteOff {
                        key: u7::new(36),
                        vel: u7::new(64),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(240),
                kind: TrackEventKind::Midi {
                    channel: DRUM_CHANNEL,
                    message: MidiMessage::NoteOn {
                        key: u7::new(56),
                        vel: u7::new(80),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(240),
                kind: TrackEventKind::Midi {
                    channel: DRUM_CHANNEL,
                    message: MidiMessage::NoteOff {
                        key: u7::new(56),
                        vel: u7::new(64),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(240),
                kind: TrackEventKind::Midi {
                    channel: DRUM_CHANNEL,
                    message: MidiMessage::NoteOn {
                        key: u7::new(38),
                        vel: u7::new(0),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            },
        ];

        let track2: Track = vec![
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: MidiMessage::NoteOn {
                        key: u7::new(60),
                        vel: u7::new(90),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(120),
                kind: TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: MidiMessage::NoteOff {
                        key: u7::new(60),
                        vel: u7::new(64),
                    },
                },
            },
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            },
        ];

        let mut bytes = Vec::new();
        Smf {
            header,
            tracks: vec![track0, track1, track2],
        }
        .write(&mut bytes)
        .unwrap();
        bytes
    }

    fn source_target_maps() -> (DrumMap, DrumMap) {
        let from = map(
            "src",
            &[
                ("Kick", 36, "kick.1"),
                ("Snare", 38, "snare.center"),
                ("Cowbell", 56, "misc.cowbell"),
            ],
        );
        let to = map(
            "dst",
            &[("Kick", 30, "kick.1"), ("Snare", 45, "snare.center")],
        );
        (from, to)
    }

    fn parse_out(bytes: &[u8]) -> Smf<'static> {
        let smf = Smf::parse(bytes).unwrap();
        let tracks = smf
            .tracks
            .iter()
            .map(|track| track.iter().map(|e| e.to_static()).collect::<Vec<_>>())
            .collect();
        Smf {
            header: smf.header,
            tracks,
        }
    }

    #[test]
    fn roundtrip_is_byte_stable_when_unchanged() {
        let smf = build_smf();
        let (from, to) = source_target_maps();
        // Identical maps: every note maps to itself.
        let (out, report) = convert(&smf, &from, &from, None, &ConvertOptions::default()).unwrap();
        assert_eq!(out, smf);
        assert!(report.rows.iter().all(|r| !r.unmapped && !r.lossy));
        let _ = to;
    }

    #[test]
    fn drum_notes_are_rewritten_other_events_byte_identical() {
        let smf = build_smf();
        let (from, to) = source_target_maps();
        let (out, report) = convert(&smf, &from, &to, None, &ConvertOptions::default()).unwrap();
        let parsed = parse_out(&out);
        let orig = parse_out(&smf);
        // Track 0 (meta only) byte-identical event-for-event.
        assert_eq!(parsed.tracks[0], orig.tracks[0]);
        // Track 2 (melody on channel 1) untouched.
        assert_eq!(parsed.tracks[2], orig.tracks[2]);
        // Format and timing preserved.
        assert_eq!(parsed.header.format, midly::Format::Parallel);
        assert_eq!(parsed.header.timing, Timing::Metrical(u15::new(480)));
        // Drums: 36 -> 30, 38 -> 45 (note-on vel 0 rewritten as note-off).
        let drums: Vec<_> = parsed.tracks[1]
            .iter()
            .filter_map(|e| match e.kind {
                TrackEventKind::Midi { message, .. } => Some(message),
                _ => None,
            })
            .collect();
        assert_eq!(drums.len(), 5);
        assert!(matches!(drums[0], MidiMessage::NoteOn { key, .. } if key.as_int() == 30));
        assert!(matches!(drums[1], MidiMessage::NoteOff { key, .. } if key.as_int() == 30));
        assert!(matches!(drums[2], MidiMessage::NoteOn { key, .. } if key.as_int() == 56));
        assert!(
            matches!(drums[4], MidiMessage::NoteOn { key, vel } if key.as_int() == 45 && vel.as_int() == 0)
        );
        // Report.
        let kick = report.rows.iter().find(|r| r.source_note == 36).unwrap();
        assert_eq!(kick.hit_count, 1);
        assert_eq!(kick.source_name, "Kick");
        assert_eq!(kick.target_note, Some(30));
        assert_eq!(kick.target_name.as_deref(), Some("Kick"));
        assert!(!kick.lossy);
        assert!(!kick.unmapped);
        let cowbell = report.rows.iter().find(|r| r.source_note == 56).unwrap();
        assert!(cowbell.unmapped);
        assert_eq!(cowbell.target_note, Some(56)); // Keep default
        assert_eq!(cowbell.hit_count, 1);
    }

    #[test]
    fn mixed_channel_track_is_not_a_drum_track() {
        let smf = parse_out(&build_smf());
        assert!(is_drum_track(&smf.tracks[1]));
        assert!(!is_drum_track(&smf.tracks[2]));
        assert!(!is_drum_track(&smf.tracks[0]));
    }

    #[test]
    fn drop_resolution_removes_unmapped_notes_and_preserves_timing() {
        let smf = build_smf();
        let (from, to) = source_target_maps();
        let opts = ConvertOptions {
            resolution: Resolution::Drop,
            ..Default::default()
        };
        let (out, report) = convert(&smf, &from, &to, None, &opts).unwrap();
        let parsed = parse_out(&out);
        let drums: Vec<_> = parsed.tracks[1]
            .iter()
            .filter_map(|e| match e.kind {
                TrackEventKind::Midi { message, .. } => Some(message),
                _ => None,
            })
            .collect();
        // Only kick + snare note events remain (36 -> 30, 38 -> 45).
        assert_eq!(drums.len(), 3);
        let cowbell = report.rows.iter().find(|r| r.source_note == 56).unwrap();
        assert!(cowbell.unmapped);
        assert_eq!(cowbell.target_note, None);
        // First drum event timing unchanged.
        assert_eq!(parsed.tracks[1][0].delta, u28::new(0));
    }

    #[test]
    fn reassign_resolution_moves_unmapped_notes() {
        let smf = build_smf();
        let (from, to) = source_target_maps();
        let opts = ConvertOptions {
            resolution: Resolution::Reassign(60),
            ..Default::default()
        };
        let (out, report) = convert(&smf, &from, &to, None, &opts).unwrap();
        let parsed = parse_out(&out);
        let drums: Vec<_> = parsed.tracks[1]
            .iter()
            .filter_map(|e| match e.kind {
                TrackEventKind::Midi { message, .. } => Some(message),
                _ => None,
            })
            .collect();
        assert_eq!(drums.len(), 5);
        assert!(matches!(drums[0], MidiMessage::NoteOn { key, .. } if key.as_int() == 30));
        assert!(matches!(drums[1], MidiMessage::NoteOff { key, .. } if key.as_int() == 30));
        assert!(matches!(drums[2], MidiMessage::NoteOn { key, .. } if key.as_int() == 60));
        assert!(matches!(drums[3], MidiMessage::NoteOff { key, .. } if key.as_int() == 60));
        let cowbell = report.rows.iter().find(|r| r.source_note == 56).unwrap();
        assert_eq!(cowbell.target_note, Some(60));
    }

    #[test]
    fn invalid_smf_is_rejected() {
        let (from, to) = source_target_maps();
        assert!(convert(&[0u8; 4], &from, &to, None, &ConvertOptions::default()).is_err());
    }
}

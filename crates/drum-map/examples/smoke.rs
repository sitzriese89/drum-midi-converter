//! End-to-end smoke test: build an Invasion-style groove in memory and
//! convert it to SD3 and to GM, printing the report tables.
//!
//! Run: cargo run -p drum-map --example smoke

use drum_map::convert::ConvertOptions;
use drum_map::midi::convert_by_id;
use midly::num::{u4, u7, u15, u24, u28};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};

fn note(on: bool, delta: u32, note: u8, vel: u8) -> TrackEvent<'static> {
    TrackEvent {
        delta: u28::new(delta),
        kind: TrackEventKind::Midi {
            channel: u4::new(9), // MIDI channel 10, the drum channel
            message: if on {
                MidiMessage::NoteOn {
                    key: u7::new(note),
                    vel: u7::new(vel),
                }
            } else {
                MidiMessage::NoteOff {
                    key: u7::new(note),
                    vel: u7::new(0),
                }
            },
        },
    }
}

fn main() {
    // Invasion-style groove: kick 24, snare 26, hat open1 45, crash 52,
    // china 65 (has no direct SD3/GM equivalent -> exercises fallbacks).
    let track: Track<'static> = vec![
        note(true, 0, 24, 100),
        note(false, 240, 24, 0),
        note(true, 0, 45, 90),
        note(false, 240, 45, 0),
        note(true, 0, 26, 110),
        note(false, 240, 26, 0),
        note(true, 0, 65, 95),
        note(false, 240, 65, 0),
        note(true, 0, 52, 100),
        note(false, 240, 52, 0),
        TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ];
    let mut smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks: vec![
            vec![TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
            }],
            track,
        ],
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes).unwrap();

    for to in ["sd3", "gm", "ezd3"] {
        let (out, report) =
            convert_by_id(&bytes, "ggd-invasion", to, &ConvertOptions::default()).unwrap();
        println!("== ggd-invasion -> {to} ({} bytes out)", out.len());
        for row in &report.rows {
            let flags = match (row.lossy, row.unmapped) {
                (true, _) => " LOSSY",
                (_, true) => " UNMAPPED",
                _ => "",
            };
            println!(
                "  {:>3} {:<16} -> {:<4} {:<20}{}",
                row.source_note,
                row.source_name,
                row.target_note.map(|n| n.to_string()).unwrap_or("-".into()),
                row.target_name.clone().unwrap_or("-".into()),
                flags
            );
        }
        // Round-trip sanity: the output must parse and contain 5 note-ons.
        let parsed = Smf::parse(&out).unwrap();
        let ons = parsed.tracks[1]
            .iter()
            .filter(|e| matches!(e.kind, TrackEventKind::Midi { message: MidiMessage::NoteOn { vel, .. }, .. } if vel > 0))
            .count();
        println!("  output parses; {ons} note-ons preserved\n");
    }
}

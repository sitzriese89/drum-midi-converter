//! Integration tests for the drum-midi-converter CLI (milestone M2).
//!
//! Fixture SMF files are generated with `midly` in a per-test tempdir; the
//! binary is exercised via `std::process::Command`. Golden-file snapshots
//! live in `tests/golden/` (plain text, no timestamps — fully deterministic).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use midly::num::{u4, u7, u15, u28};
use midly::{Format, Header, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};

const EXE: &str = env!("CARGO_BIN_EXE_drum-midi-converter");

/// Fixture: GM-mapped drum hits as (note, velocity) pairs, one note-off 240
/// ticks after each note-on, on the drum channel (10).
const FIXTURE_HITS: &[(u8, u8)] = &[
    (36, 100), // kick: 3 hits
    (36, 100),
    (36, 100),
    (38, 90), // snare: 2 hits
    (38, 90),
    (42, 70), // closed hat: 2 hits (LOSSY via documented fallback)
    (42, 70),
    (49, 110), // crash: 1 hit
    (56, 80),  // cowbell: 1 hit (unmapped in ggd-invasion)
];

// --- helpers ---------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("dmc-cli-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(EXE).args(args).output().unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn assert_failure(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "expected failure, but exited 0\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn golden(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    fs::read_to_string(path).unwrap_or_else(|e| panic!("golden file {name} missing: {e}"))
}

/// Build a single-track SMF with drum-channel note on/off pairs.
fn drum_smf(hits: &[(u8, u8)]) -> Vec<u8> {
    let header = Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480)));
    let mut track: Track = Vec::new();
    for &(key, vel) in hits {
        track.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Midi {
                channel: u4::new(9),
                message: MidiMessage::NoteOn {
                    key: u7::new(key),
                    vel: u7::new(vel),
                },
            },
        });
        track.push(TrackEvent {
            delta: u28::new(240),
            kind: TrackEventKind::Midi {
                channel: u4::new(9),
                message: MidiMessage::NoteOff {
                    key: u7::new(key),
                    vel: u7::new(64),
                },
            },
        });
    }
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
    });
    let mut bytes = Vec::new();
    Smf {
        header,
        tracks: vec![track],
    }
    .write(&mut bytes)
    .unwrap();
    bytes
}

/// Unique drum-channel note numbers present in a parsed SMF (note-on vel > 0).
fn drum_notes(bytes: &[u8]) -> Vec<u8> {
    let smf = Smf::parse(bytes).unwrap();
    let mut notes = Vec::new();
    for track in &smf.tracks {
        for event in track {
            if let TrackEventKind::Midi { channel, message } = event.kind
                && channel == u4::new(9)
                && let MidiMessage::NoteOn { key, vel } = message
                && vel.as_int() > 0
            {
                notes.push(key.as_int());
            }
        }
    }
    notes.sort_unstable();
    notes.dedup();
    notes
}

fn write_fixture(dir: &TempDir, name: &str) -> PathBuf {
    let path = dir.path(name);
    fs::write(&path, drum_smf(FIXTURE_HITS)).unwrap();
    path
}

// --- tests ------------------------------------------------------------------

#[test]
fn list_maps_matches_golden() {
    let output = run(&["--list-maps"]);
    assert_success(&output);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, golden("list-maps.txt"));
}

#[test]
fn convert_writes_parseable_output() {
    let dir = TempDir::new("convert");
    let input = write_fixture(&dir, "in.mid");
    let out = dir.path("out.mid");
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Wrote"),
        "should confirm the output file: {stdout}"
    );
    // GM 36/38/42/49 -> Invasion 24/26/43/52; cowbell 56 kept as-is (default).
    assert_eq!(
        drum_notes(&fs::read(&out).unwrap()),
        vec![24, 26, 43, 52, 56]
    );
}

#[test]
fn report_matches_golden() {
    let dir = TempDir::new("report");
    let input = write_fixture(&dir, "in.mid");
    let out = dir.path("out.mid");
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "-o",
        out.to_str().unwrap(),
        "--report",
    ]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, golden("report-gm-to-invasion.txt"));
    // The output file must still be written alongside the report.
    assert!(out.exists());
}

#[test]
fn dry_run_report_without_output() {
    let dir = TempDir::new("dry-run");
    let input = write_fixture(&dir, "in.mid");
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "--report",
    ]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, golden("report-gm-to-invasion.txt"));
    // Dry run: nothing was written next to the input.
    assert!(!dir.path("out.mid").exists());
    assert!(!dir.path("in-converted.mid").exists());
}

#[test]
fn unmapped_drop_removes_unmapped_notes() {
    let dir = TempDir::new("drop");
    let input = write_fixture(&dir, "in.mid");
    let out = dir.path("out.mid");
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "--unmapped",
        "drop",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_success(&output);
    // Cowbell (56) gone; everything else converted.
    assert_eq!(drum_notes(&fs::read(&out).unwrap()), vec![24, 26, 43, 52]);
}

#[test]
fn unmapped_reassign_moves_unmapped_notes() {
    let dir = TempDir::new("reassign");
    let input = write_fixture(&dir, "in.mid");
    let out = dir.path("out.mid");
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "--unmapped",
        "reassign=60",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_success(&output);
    // Cowbell (56) reassigned to 60; everything else converted.
    assert_eq!(
        drum_notes(&fs::read(&out).unwrap()),
        vec![24, 26, 43, 52, 60]
    );
}

#[test]
fn reassign_without_value_is_rejected() {
    let output = run(&["--unmapped", "reassign"]);
    let stderr = assert_failure(&output);
    assert!(
        stderr.contains("reassign"),
        "should explain the missing value: {stderr}"
    );
}

#[test]
fn reassign_out_of_range_is_rejected() {
    let output = run(&["--unmapped", "reassign=200"]);
    let stderr = assert_failure(&output);
    assert!(
        stderr.contains("0-127"),
        "should explain the valid range: {stderr}"
    );
}

#[test]
fn unknown_map_id_fails_with_clear_message() {
    let dir = TempDir::new("unknown-map");
    let input = write_fixture(&dir, "in.mid");
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "no-such-map",
        "-o",
        dir.path("out.mid").to_str().unwrap(),
    ]);
    let stderr = assert_failure(&output);
    assert!(
        stderr.contains("unknown map id") && stderr.contains("no-such-map"),
        "should name the unknown map: {stderr}"
    );
}

#[test]
fn non_midi_input_fails_with_clear_message() {
    let dir = TempDir::new("non-midi");
    let input = dir.path("garbage.mid");
    fs::write(&input, b"this is not a MIDI file at all").unwrap();
    let output = run(&[
        input.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "-o",
        dir.path("out.mid").to_str().unwrap(),
    ]);
    let stderr = assert_failure(&output);
    assert!(
        stderr.contains("invalid SMF"),
        "should report the parse failure: {stderr}"
    );
}

#[test]
fn missing_input_file_fails_with_clear_message() {
    let dir = TempDir::new("missing-file");
    let missing = dir.path("does-not-exist.mid");
    let output = run(&[
        missing.to_str().unwrap(),
        "--from",
        "gm",
        "--to",
        "ggd-invasion",
        "-o",
        dir.path("out.mid").to_str().unwrap(),
    ]);
    let stderr = assert_failure(&output);
    assert!(
        stderr.contains("failed to read input file"),
        "should report the read failure: {stderr}"
    );
}

# ADR — drum-midi-converter

Architecture decisions, newest first. Q-IDs trace to the design interview
summarized in SPEC.md.

## ADR-001 — Semantic-tag matching instead of N×N pair tables (Q9)

Each note in each drum map carries a semantic articulation tag
(`hihat.closed.edge`, `snare.rimshot`, ...). Conversions are derived by
matching tags, with small curated per-pair override tables only where the
generic fallback is wrong.

Rationale: with 7 built-in maps, hand-authoring every directed pair means
42 tables — a maintenance trap. Tag matching makes the Nth map cost O(1)
tables instead of O(N), and custom user maps (Q6) get conversions to every
built-in for free.

## ADR-002 — Pure Rust core compiled to WASM; Tauri is filesystem-only (Q7)

Mirrors chords-and-scales: `crates/drum-map` holds all logic, compiled to
WASM and called directly from the Svelte frontend, including inside the
Tauri shell. Tauri commands exist only for reading/writing files and
persisting user data.

Rationale: the web build then needs only a different filesystem layer;
logic is tested once, in Rust; the same core powers the CLI.

## ADR-003 — Detection is a hint, not a decision (Q1)

Source map is chosen by the user from a dropdown. A heuristic note-usage
hint ("looks like GGD Invasion") and a mismatch warning help catch wrong
picks, but the app never silently decides. Full auto-detection is a stretch
goal.

Rationale: detection can be wrong; silent wrongness in a conversion tool
corrupts user work.

## ADR-004 — Unmapped notes are kept, not dropped (Q3)

Notes with no mapping entry pass through unchanged by default and are listed
in the report with per-note keep/drop/reassign overrides.

Rationale: silent data loss is worse than a visible warning; the report is
where wrong source-map picks are caught.

## ADR-005 — Fallbacks are documented and reported (Q2)

Where the target map lacks an articulation, a documented per-family rule
applies (e.g. collapse hi-hat openness levels). Every firing appears in the
report as LOSSY.

Rationale: lossy conversions must never be silent.

## ADR-006 — Velocities pass through in v1 (Q11)

No velocity remapping in v1; per-pair velocity curves are a high-priority
v1.x feature needing per-library data and a curve UI.

## ADR-007 — midly for MIDI I/O; non-drum data preserved

Parsing/serialization via `midly`. SMF format type, tempo map, time
signatures, markers and all non-note events are preserved; only note events
on converted drum tracks change.

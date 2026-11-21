# drum-midi-converter — Specification

A desktop and web tool for converting drum MIDI files between the note maps
of popular drum libraries (GetGood Drums, Toontrack, RS Drums, ...).

Derived from a design interview. Decision IDs (Q1–Q13) refer to that
interview and are kept for traceability.

---

## 1. Product

| Topic | Decision |
|---|---|
| Audience (Q8) | Drummers/producers switching drum libraries; open source. |
| Platforms (Q7) | Windows, macOS, Linux desktop (Tauri 2) plus a web build from the same code. |
| License (Q12) | MIT / Apache-2.0 dual license. |
| Naming (Q12) | Library names appear as identifiers only; no vendor logos or branding assets. |
| File scope (Q4) | Drum-only .mid files, single file at a time (v1). Core stays multitrack- and batch-ready as extensions. |

---

## 2. Architecture

### 2.1 Repository layout (Q7)

Monorepo, Cargo workspace + pnpm — mirrors the chords-and-scales setup:

```
crates/
  drum-map/        # Pure Rust core. No UI/IO dependencies beyond file bytes.
  drum-map-wasm/   # wasm-bindgen bindings exposing drum-map to JS/TS.
  drum-map-cli/    # Headless CLI binary (Q13).
app/               # Svelte + TypeScript + Vite frontend
  src-tauri/       # Tauri 2 shell
data/maps/         # Built-in drum maps (TOML), embedded at build time
```

### 2.2 Core boundary (Q7)

- All conversion logic (MIDI parse/write, map loading, semantic matching,
  override resolution, report generation) lives in `crates/drum-map`.
- The core is compiled to **WASM** and called **directly from the frontend**,
  also inside Tauri. **Tauri commands are used only for filesystem work**:
  reading the input file, writing the output, persistence of custom maps and
  settings.
- MIDI parsing/serialization uses the `midly` crate. Non-drum-track events
  and file structure (SMF format type, tempo map, time signatures, markers)
  are preserved; only note events on the selected drum track(s) change.

### 2.3 Frontend (Q7)

- Svelte + TypeScript + Vite. Diagrams rendered as SVG.
- CLI (Q13): `drum-midi-converter in.mid --from <map> --to <map> -o out.mid`.

### 2.4 CI (Q12)

- GitHub Actions on every PR: `cargo test`, `cargo clippy`, `cargo fmt --check`,
  frontend lint/type-check/tests (once the frontend exists).
- On tag: Tauri release builds for Windows, macOS, Linux. Web build deployed
  separately (static site).

### 2.5 Testing

- `drum-map` crate: unit tests per built-in map (parse, tag coverage) plus
  property-based tests (conversion round-trips preserve timing, event order
  and non-drum data; applying a map then its inverse is lossless for
  non-lossy notes).

---

## 3. Drum maps

### 3.1 Built-in catalog v1 (Q8)

One TOML file per product; maps differ per product, not per vendor:

- GetGood Drums Invasion
- GetGood Drums Modern & Massive
- GetGood Drums One Kit Wonder
- Toontrack Superior Drummer 3 (extended map)
- Toontrack EZdrummer 2/3
- RS Drums (one kit; more added over time)
- General MIDI (floor / universal fallback)

### 3.2 Semantic articulation tags (Q9)

Each note in every map is tagged with a semantic articulation ID, e.g.
`kick.1`, `snare.center`, `snare.rimshot`, `hihat.closed.edge`,
`hihat.open.3`, `crash.1`. A conversion between two maps is **derived by
matching tags**; small curated per-pair override tables exist only where the
generic rule gives the wrong answer.

### 3.3 Fallback rules (Q2)

When the target map has no exact tag, a documented per-family fallback fires,
e.g. collapse hi-hat openness levels to the nearest available
(`hihat.open.4` → `hihat.open.3` if only three levels exist), rimshot →
sidestick, cross-stick → center hit, etc. Every fallback that fires during a
conversion appears in the report as LOSSY.

### 3.4 Custom maps (Q6)

Users can create/edit their own maps (TOML, same schema as built-ins) stored
in the OS app-data directory, with import/export for sharing. Built-in maps
are read-only and embedded in the binary.

---

## 4. Conversion flow

### 4.1 Source/target selection (Q1)

User picks source and target maps from dropdowns. The app computes a
detection hint ("this file looks like GGD Invasion") from note-usage
heuristics and warns when the file's used notes don't fit the chosen source
map. Full auto-detection is a stretch goal, not a requirement.

### 4.2 Unmapped notes (Q3)

Notes with no mapping entry are kept unchanged by default and listed in the
pre-export report, where the user can override per note: keep / drop /
reassign to any target note.

### 4.3 Report UI (Q10)

Table-first: one row per used note — instrument name, hit count, mapped
target, LOSSY badge where a fallback fired, override dropdown for unmapped
and lossy notes. An SVG drum-kit diagram preview is a stretch goal; there is
no piano roll.

### 4.4 Velocities (Q11)

Pass-through in v1. Per-pair velocity curve adjustment is a high-priority
v1.x feature.

### 4.5 Output (Q13)

Save dialog and **drag-out** (drag the converted file from the app straight
into a DAW/explorer) in v1.

---

## 5. Roadmap anchors

v1: dropdowns + hint, 7 built-in maps, tag matching + overrides, report UI,
save + drag-out, CLI, desktop + web.
v1.x: velocity curves, in-app audition (WebAudio A/B), round-robin
alternates, batch conversion, SVG kit preview, multitrack support.
Stretch: full source-map auto-detection.

Details and milestones: docs/roadmap.md.

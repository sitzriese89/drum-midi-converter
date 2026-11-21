# Roadmap — drum-midi-converter

Status: M0 (skeleton).

## M0 — Repository skeleton (current)

- [x] SPEC.md, ADR.md, roadmap, licenses, workspace layout
- [ ] Workspace compiles: `cargo build`, `cargo test` green

## M1 — Core crate (`crates/drum-map`)

- [ ] TOML map schema: notes, semantic tags, fallback rules, per-pair overrides
- [ ] Built-in catalog v1 (Q8): GGD Invasion, GGD M&M, GGD OKW, SD3, EZD2/3,
      RS Drums (one kit), General MIDI — embedded at build time
- [ ] MIDI parse/serialize via `midly`; preserve non-drum data (ADR-007)
- [ ] Tag-matching converter + override resolution + LOSSY report data model
- [ ] Unmapped-note passthrough with keep/drop/reassign resolution (Q3)
- [ ] Detection heuristic + confidence hint (Q1)
- [ ] Unit tests per map; property tests for round-trips

## M2 — CLI (`crates/drum-map-cli`)

- [ ] `drum-midi-converter in.mid --from <map> --to <map> -o out.mid`
- [ ] `--report` prints the conversion report; `--list-maps`
- [ ] Snapshot tests on fixture MIDI files

## M3 — Frontend + Tauri (mirrors chords-and-scales)

- [ ] Svelte + TS + Vite app; `drum-map-wasm` bindings
- [ ] Dropdowns with detection hint + mismatch warning
- [ ] Table-first report UI with LOSSY badges and per-note overrides (Q10)
- [ ] Save dialog + drag-out to DAW/explorer (Q13)
- [ ] Tauri 2 shell: filesystem commands only (ADR-002)
- [ ] Custom map editor + import/export (Q6)

## M4 — Web build

- [ ] Static web build from the same frontend (WASM core)
- [ ] File handling without Tauri (browser file input / download)

## M5 — v1.x features (priority order)

1. Velocity curves per pair (Q11 — high priority)
2. In-app audition: WebAudio A/B source vs converted (Q13)
3. Round-robin alternates for missing articulations (Q13)
4. Batch conversion (Q5)
5. SVG kit-diagram preview (Q10)
6. Multitrack file support (Q4)

## Stretch

- Full source-map auto-detection (Q1)

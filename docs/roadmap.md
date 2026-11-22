# Roadmap — drum-midi-converter

Status: M3 part 1 (WASM + frontend + Tauri) done. Next: M3 map editor (Q6).

## M0 — Repository skeleton

- [x] SPEC.md, ADR.md, roadmap, licenses, workspace layout
- [x] Workspace compiles: `cargo build`, `cargo test` green

## M1 — Core crate (`crates/drum-map`)

- [x] TOML map schema: notes, semantic tags, fallback rules, per-pair overrides
- [x] Built-in catalog v1 (Q8): GGD Invasion, GGD M&M, GGD OKW (Modern Fusion),
      SD3, EZD3, RS Drums (Fairview), General MIDI — embedded at build time.
      Note data is UNOFFICIAL (Drumverter-sourced, see data/research/);
      tag vocabulary in docs/tag-taxonomy.md
- [x] MIDI parse/serialize via `midly`; preserve non-drum data (ADR-007)
- [x] Tag-matching converter + override resolution + LOSSY report data model
- [x] Unmapped-note passthrough with keep/drop/reassign resolution (Q3)
- [x] Detection heuristic + confidence hint (Q1)
- [x] Unit tests per map; property tests for round-trips (41 tests green;
      end-to-end check: `cargo run -p drum-map --example smoke`)

## M2 — CLI (`crates/drum-map-cli`)

- [x] `drum-midi-converter in.mid --from <map> --to <map> -o out.mid`
- [x] `--report` prints the conversion report; `--list-maps`
- [x] `--unmapped keep|drop|reassign=N` (Q3 resolution exposure)
- [x] Snapshot tests on fixture MIDI files (golden files, 11 integration tests)

## M3 — Frontend + Tauri (mirrors chords-and-scales)

- [x] Svelte + TS + Vite app; `drum-map-wasm` bindings
- [x] Dropdowns with detection hint + mismatch warning
- [x] Table-first report UI with LOSSY badges and per-note overrides (Q10)
- [x] Save dialog + drag-out to DAW/explorer (Q13)
- [x] Tauri 2 shell: filesystem commands only (ADR-002)
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

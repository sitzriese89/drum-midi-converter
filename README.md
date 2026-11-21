# drum-midi-converter

Converts drum MIDI files between the note maps of popular drum libraries
(GetGood Drums, Toontrack Superior/EZdrummer, RS Drums, General MIDI, ...),
on desktop (Windows, macOS, Linux) and on the web.

What it does: [SPEC.md](SPEC.md). Plan and status: [docs/roadmap.md](docs/roadmap.md).
Design decisions: [ADR.md](ADR.md).

## Layout

```
crates/
  drum-map/        # Pure Rust core: MIDI parsing, map matching, conversion, report
  drum-map-wasm/   # wasm-bindgen bindings exposing drum-map to JS/TS
  drum-map-cli/    # Headless CLI binary
app/               # Svelte + TypeScript + Vite frontend (added in a later milestone)
  src-tauri/       # Tauri 2 shell (filesystem only)
data/maps/         # Built-in drum maps (TOML), embedded into the binary
```

## Development

```
cargo build
cargo test
cargo run -p drum-map-cli -- --help
```

Frontend setup arrives with milestone M3 (see docs/roadmap.md).

## License

MIT / Apache-2.0, at your option. See LICENSE-MIT and LICENSE-APACHE.

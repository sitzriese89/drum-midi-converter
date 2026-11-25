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

Frontend (needs Node 24, pnpm and the `wasm32-unknown-unknown` Rust target):

```
cd app
pnpm install
pnpm dev           # web build in the browser, http://localhost:1420
pnpm tauri dev     # desktop app
pnpm test          # WASM build + vitest
pnpm build         # static web build in app/dist/ (relative paths: host it anywhere)
```

The desktop and web builds share one frontend. On the web, MIDI files come in through
the file picker or a drop and leave as a browser download (no drag-out to a DAW), and
user maps are kept in the browser's localStorage, so export a map to keep a copy.
Pushing a `v*` tag deploys the web build to GitHub Pages (`.github/workflows/web.yml`).

## License

MIT / Apache-2.0, at your option. See LICENSE-MIT and LICENSE-APACHE.

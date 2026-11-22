// The drum-map core, compiled to WASM from crates/drum-map-wasm by `pnpm wasm` (ADR-002).
// Components import drum-map functions from here, never from ./wasm directly. The generated
// module keeps the Rust snake_case names; the camelCase aliases below are the app's API.
import init from './wasm/drum_map_wasm.js';
import {
  builtin_map,
  convert_midi,
  detect_hints,
  list_maps,
  map_notes,
  map_to_toml,
  parse_map_toml,
  source_coverage,
  validate_map,
  version as versionRaw,
} from './wasm/drum_map_wasm.js';

export const version = versionRaw;
export const listMaps = list_maps;
export const mapNotes = map_notes;
export const detectHints = detect_hints;
export const sourceCoverage = source_coverage;
export const convertMidi = convert_midi;

// User map editor (crates/drum-map-wasm/src/user_maps.rs, SPEC §3.4).
export const validateMap = validate_map;
export const mapToToml = map_to_toml;
export const parseMapToml = parse_map_toml;
export const builtinMap = builtin_map;
export type {
  MapFallbackRule,
  MapFull,
  MapInput,
  MapIssue,
  MapNote,
  MapParseResult,
} from './wasm/drum_map_wasm.js';

// Built-in catalog (crates/drum-map-wasm/src/catalog.rs).
export type { MapInfo, NoteName } from './wasm/drum_map_wasm.js';

// Detection hints (crates/drum-map-wasm/src/detect.rs, ADR-003). A hint, never a decision.
export type { Candidate } from './wasm/drum_map_wasm.js';

// Conversion (crates/drum-map-wasm/src/convert.rs). Per-note overrides for unmapped notes
// (ADR-004): `{ 56: { kind: 'drop' } }` or `{ 56: { kind: 'reassign', note: 60 } }`.
export type { ConvertResult, NoteRow, Report } from './wasm/drum_map_wasm.js';

let ready: Promise<void> | undefined;

/** Loads the WASM module; the core must be loaded before any component calls it. */
export function initDrumMap(wasm?: BufferSource): Promise<void> {
  if (!ready) {
    ready = init(wasm ? { module_or_path: wasm } : undefined).then(() => {});
  }
  return ready;
}

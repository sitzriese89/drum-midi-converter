// Conversion flow state (SPEC §4.1–§4.3, ADR-003, ADR-004). Svelte 5 runes; the components
// hold the rendering, this holds the flow.

import { SvelteMap } from 'svelte/reactivity';

import {
  convertMidi,
  detectHints,
  listMaps,
  mapNotes,
  sourceCoverage,
  type Candidate,
  type MapInfo,
  type NoteName,
  type Report,
} from './drum-map';
import { DEFAULT_CHOICE, type OverrideChoice, overridesPayload } from './overrides';
import { type MidiFile, isTauri, pickMidiFile, saveMidiFile, writeTempFile } from './files';

/** Everything the conversion flow needs: file, picks, hint and the last conversion. */
export class Converter {
  /** The loaded input file, null until one is picked or dropped in. */
  file = $state<MidiFile | null>(null);
  /** All built-in maps, in catalog order. */
  maps = $state<MapInfo[]>([]);
  /** Chosen source and target map ids. */
  fromId = $state('');
  toId = $state('');
  /** Ranked detection candidates for the loaded file (ADR-003). */
  hints = $state<Candidate[]>([]);
  /** Fraction of the used notes the chosen source map covers, null before a file is loaded. */
  coverage = $state<number | null>(null);
  /** The target map's full note vocabulary, for reassign overrides (SPEC §4.2). */
  targetNotes = $state<NoteName[]>([]);
  /** Per-note overrides for unmapped and lossy notes, keyed by source note (ADR-004, §4.3). */
  overrides = $state<SvelteMap<number, OverrideChoice>>(new SvelteMap());
  /** The last conversion's report, null before the first conversion. */
  report = $state<Report | null>(null);
  /** The last conversion's output bytes, null before the first conversion. */
  converted = $state<Uint8Array | null>(null);
  /** Error message from a failed load or conversion. */
  error = $state<string | null>(null);

  /** The hint to display, if any: the best candidate with a confident-enough score. */
  hint = $derived.by(() => {
    const best = this.hints[0];
    return best && best.confidence >= 0.5 ? best : null;
  });

  /** True when the file's used notes don't all fit the chosen source map. */
  mismatch = $derived(this.coverage !== null && this.coverage < 1.0);

  /** Loads the catalog once at startup; `extra` are the user's own maps (SPEC §3.4). */
  async loadCatalog(extra: MapInfo[] = []) {
    this.maps = [...(await listMaps()), ...extra];
    if (!this.fromId) {
      this.fromId = this.maps[0]?.id ?? '';
    }
    if (!this.toId) {
      const target = this.maps[1] ?? this.maps[0];
      this.toId = target?.id ?? '';
    }
    await this.loadTargetNotes();
  }

  /**
   * Loads a MIDI file and computes the detection hint (ADR-003: a hint,
   * never a decision — the source dropdown is not touched; the user picks,
   * optionally via the hint chip's Apply button).
   */
  async loadFile(file: MidiFile) {
    this.error = null;
    this.file = file;
    this.report = null;
    this.converted = null;
    this.overrides = new SvelteMap();
    try {
      this.hints = await detectHints(file.bytes);
      this.coverage = this.fromId ? ((await sourceCoverage(file.bytes, this.fromId)) ?? 1.0) : null;
    } catch (e) {
      this.hints = [];
      this.coverage = null;
      this.error = `Could not read ${file.name}: ${message(e)}`;
    }
  }

  /** Opens the native open dialog (Tauri only; the web build picks through a file input). */
  async pickFile() {
    const picked = await pickMidiFile();
    if (picked) await this.loadFile(picked);
  }

  /** Re-checks the coverage after the user changes the source map. */
  async setFromId(id: string) {
    this.fromId = id;
    if (this.file) {
      this.coverage = (await sourceCoverage(this.file.bytes, id)) ?? 1.0;
    }
  }

  /** Sets the target map and refreshes the full reassign vocabulary (SPEC §4.2). */
  async setToId(id: string) {
    this.toId = id;
    await this.loadTargetNotes();
  }

  /** Loads the full note vocabulary of the chosen target map. */
  async loadTargetNotes() {
    if (!this.toId) {
      this.targetNotes = [];
      return;
    }
    try {
      this.targetNotes = await mapNotes(this.toId);
    } catch {
      this.targetNotes = [];
    }
  }

  /** Sets the override for one note (the report's dropdowns call this). */
  setOverride(note: number, choice: OverrideChoice) {
    if (choice.kind === DEFAULT_CHOICE.kind) {
      this.overrides.delete(note);
    } else {
      this.overrides.set(note, choice);
    }
  }

  /** Runs the conversion; re-running picks up the current overrides (ADR-004). */
  async convert() {
    if (!this.file || !this.fromId || !this.toId) return;
    this.error = null;
    try {
      const result = await convertMidi(
        this.file.bytes,
        this.fromId,
        this.toId,
        overridesPayload(this.overrides),
      );
      this.report = result.report;
      this.converted = result.midi;
    } catch (e) {
      this.error = `Conversion failed: ${message(e)}`;
    }
  }

  /** Saves the converted file (native save dialog under Tauri, browser download on the web). */
  async save(name: string) {
    if (!this.converted) return null;
    return saveMidiFile(name, this.converted);
  }

  /** Writes the converted file to a temp path for drag-out (Q13). Tauri only. */
  async dragOut(name: string) {
    if (!this.converted || !isTauri()) return null;
    return writeTempFile(name, this.converted);
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

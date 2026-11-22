// The map editor flow (SPEC §3.4, Q6): edit a structured map draft, validate it against
// the built-in schema (field-by-field issues from the core), save it to the app-data
// folder, import a shared TOML, export the draft, and duplicate a read-only built-in as
// a starting point. The components hold the rendering, this holds the state.

import {
  builtinMap,
  mapToToml,
  parseMapToml,
  validateMap,
  type MapFull,
  type MapInfo,
  type MapInput,
  type MapIssue,
  type MapParseResult,
} from './drum-map';
import { type UserMaps } from './user-maps.svelte';

export class MapEditor {
  /** The built-in catalog, for the duplicate-as-starting-point list. */
  builtins = $state<MapInfo[]>([]);
  /** The map under construction, null until the user starts one. */
  draft = $state<MapInput | null>(null);
  /** Validation or parse problems for the current draft, field-by-field. */
  issues = $state<MapIssue[]>([]);
  /** Feedback for the last completed action (saved, imported, deleted). */
  status = $state<string | null>(null);
  /** Error message from a failed load/save/delete. */
  error = $state<string | null>(null);

  constructor(
    /** The user maps store: list, read, write, delete, export. */
    readonly userMaps: UserMaps,
  ) {}

  /** Loads the built-in catalog and the stored user maps. */
  async load(builtins: MapInfo[]) {
    this.builtins = builtins;
    await this.userMaps.load();
  }

  /** Starts a fresh one-note draft. */
  newMap() {
    this.draft = {
      id: '',
      name: '',
      vendor: '',
      description: null,
      notes: [{ note: 36, name: 'Kick', tag: 'kick.1' }],
      fallback: null,
    };
    this.clearMessages();
  }

  /** Loads a read-only built-in as an editable copy. The id is suffixed
   * `-copy`, `-copy-2`, … so a second duplicate never overwrites the first;
   * uniqueness is checked against the stored user-map ids (the id is also
   * the file name, SPEC §3.4). */
  duplicateBuiltin(id: string) {
    const full: MapFull = builtinMap(id);
    // A plain array read-only check (the ids come from $state, never mutated).
    const taken = this.userMaps.maps.map((map) => map.id);
    let uniqueId = `${full.id}-copy`;
    for (let n = 2; taken.includes(uniqueId); n += 1) uniqueId = `${full.id}-copy-${n}`;
    this.draft = {
      id: uniqueId,
      name: `${full.name} (copy)`,
      vendor: full.vendor,
      description: full.description,
      notes: full.notes.map((note: MapFull['notes'][number]) => ({ ...note })),
      fallback:
        full.fallback?.map((rule: MapFull['fallback'] extends (infer R)[] | null ? R : never) => ({
          ...rule,
        })) ?? null,
    };
    this.clearMessages();
  }

  /** Loads a stored user map for editing. */
  async editStored(id: string) {
    this.clearMessages();
    const text = await this.userMaps.read(id);
    if (text === null) {
      this.error = `Map "${id}" could not be read.`;
      return;
    }
    this.loadToml(text, `Map "${id}" loaded.`);
  }

  /** Parses an imported TOML into the draft; parse errors are shown, never swallowed. */
  importToml(text: string, label: string) {
    this.clearMessages();
    this.loadToml(text, `Imported ${label}.`);
  }

  private loadToml(text: string, ok: string) {
    const result = parseMapToml(text);
    if (!result.map) {
      this.issues = result.issues;
      this.status = null;
      return;
    }
    const full = result.map;
    this.draft = {
      id: full.id,
      name: full.name,
      vendor: full.vendor,
      description: full.description,
      notes: full.notes.map((note: MapFull['notes'][number]) => ({ ...note })),
      fallback:
        full.fallback?.map((rule: MapFull['fallback'] extends (infer R)[] | null ? R : never) => ({
          ...rule,
        })) ?? null,
    };
    this.issues = [];
    this.status = ok;
  }

  /** Adds an empty note row on the lowest free MIDI number. */
  addNote() {
    if (!this.draft) return;
    let note = 35;
    while (note < 127 && this.draft.notes.some((n) => n.note === note)) note += 1;
    this.draft.notes = [...this.draft.notes, { note, name: '', tag: '' }];
  }

  /** Removes one note row. */
  removeNote(note: number) {
    if (!this.draft) return;
    this.draft.notes = this.draft.notes.filter((n) => n.note !== note);
  }

  /** Adds an empty fallback rule. */
  addFallbackRule() {
    if (!this.draft) return;
    this.draft.fallback = [...(this.draft.fallback ?? []), { from: '', to: '' }];
  }

  /** Removes one fallback rule. */
  removeFallbackRule(index: number) {
    if (!this.draft) return;
    const rules = (this.draft.fallback ?? []).filter((_, i) => i !== index);
    this.draft.fallback = rules.length > 0 ? rules : null;
  }

  /** Validates the draft; the issues are kept for the UI and the result returned. */
  validate(): boolean {
    if (!this.draft) return false;
    const checked = this.checked(this.draft);
    this.issues = checked.issues;
    return checked.map !== null;
  }

  /**
   * Validates and saves the draft. Invalid maps are refused with their issues:
   * nothing is written, no note silently dropped.
   */
  async save(): Promise<boolean> {
    if (!this.draft) return false;
    const checked = this.checked(this.draft);
    if (!checked.map) {
      this.issues = checked.issues;
      this.status = null;
      return false;
    }
    this.issues = [];
    const toml = mapToToml({
      id: checked.map.id,
      name: checked.map.name,
      vendor: checked.map.vendor,
      description: checked.map.description,
      notes: checked.map.notes,
      fallback: checked.map.fallback,
    });
    await this.userMaps.save(checked.map.id, toml);
    if (this.userMaps.error) {
      this.error = this.userMaps.error;
      return false;
    }
    this.status = `Map "${checked.map.id}" saved.`;
    return true;
  }

  /** Exports the draft as a shared TOML (save dialog or browser download). */
  async exportMap(): Promise<string | null> {
    if (!this.draft) return null;
    if (!this.validate()) return null;
    const toml = mapToToml({
      id: this.draft.id,
      name: this.draft.name,
      vendor: this.draft.vendor,
      description: this.draft.description,
      notes: this.draft.notes,
      fallback: this.draft.fallback,
    });
    return this.userMaps.exportFile(this.draft.id, toml);
  }

  /** Deletes a stored user map. Built-ins are never deletable (read-only). */
  async deleteMap(id: string) {
    await this.userMaps.delete(id);
    if (this.userMaps.error) this.error = this.userMaps.error;
    else this.status = `Map "${id}" deleted.`;
  }

  /** Validates `draft` in the core; never throws — problems come back as issues. */
  private checked(draft: MapInput): MapParseResult {
    try {
      return validateMap({
        id: draft.id,
        name: draft.name,
        vendor: draft.vendor,
        description: draft.description,
        notes: draft.notes.map((n) => ({ ...n })),
        fallback: draft.fallback?.map((r) => ({ ...r })) ?? null,
      }) as MapParseResult;
    } catch (error) {
      return { map: null, issues: [{ field: 'map', message: message(error) }] };
    }
  }

  private clearMessages() {
    this.issues = [];
    this.status = null;
    this.error = null;
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

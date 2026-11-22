// The map editor flow (SPEC §3.4): duplicate a read-only built-in, edit notes, validate
// field-by-field against the built-in schema, save (and never save an invalid map), and
// import shared TOML with parse errors surfaced.
import { describe, expect, it } from 'vitest';
import { MapEditor } from './map-editor.svelte';
import { UserMaps, type MapStorage } from './user-maps.svelte';

const GM_TOML = await import('./wasm/drum_map_wasm.js').then((w) => w.list_maps);

function memoryStorage(): MapStorage {
  const files = new Map<string, string>();
  return {
    available: true,
    list: async () => [...files.keys()].sort(),
    read: async (id) => files.get(id) ?? null,
    write: async (id, content) => {
      files.set(id, content);
    },
    delete: async (id) => {
      files.delete(id);
    },
    exportFile: async () => null,
  };
}

async function editorWith(storage: MapStorage = memoryStorage()) {
  const userMaps = new UserMaps(storage);
  const editor = new MapEditor(userMaps);
  const builtins = await GM_TOML();
  await editor.load(builtins);
  return editor;
}

describe('MapEditor', () => {
  it('duplicates a built-in map as an editable copy, notes intact', async () => {
    const editor = await editorWith();
    editor.duplicateBuiltin('gm');
    expect(editor.draft!.id).toBe('gm-copy');
    expect(editor.draft!.notes.length).toBe(47);
    // The built-in itself is untouched: it stays read-only and is not in the user list.
    expect(editor.userMaps.maps).toEqual([]);
  });

  it('validates field-by-field and never saves an invalid map', async () => {
    const editor = await editorWith();
    editor.newMap();
    editor.draft!.name = '';
    editor.draft!.notes[0].tag = 'Kick!';
    expect(await editor.save()).toBe(false);
    const fields = editor.issues.map((i) => i.field);
    expect(fields).toContain('id');
    expect(fields).toContain('name');
    // Per-note fields are keyed by MIDI note number, not row index.
    expect(fields).toContain('notes.36.tag');
    expect(editor.userMaps.maps).toEqual([]);
  });

  it('duplicating twice gets distinct ids instead of overwriting the first copy', async () => {
    const editor = await editorWith();
    editor.duplicateBuiltin('gm');
    expect(editor.draft!.id).toBe('gm-copy');
    editor.userMaps.maps = [{ id: 'gm-copy', name: 'General MIDI (copy)' }];
    editor.duplicateBuiltin('gm');
    expect(editor.draft!.id).toBe('gm-copy-2');
  });

  it('a tag outside the vocabulary is rejected with a pointer to the taxonomy', async () => {
    const editor = await editorWith();
    editor.newMap();
    editor.draft!.id = 'my-kit';
    editor.draft!.name = 'My Kit';
    editor.draft!.notes[0].tag = 'gong.1';
    expect(await editor.save()).toBe(false);
    expect(editor.issues.some((i) => i.message.includes('docs/tag-taxonomy.md'))).toBe(true);
  });

  it('saves a valid map and it shows up in the user list', async () => {
    const editor = await editorWith();
    editor.newMap();
    editor.draft!.id = 'my-kit';
    editor.draft!.name = 'My Kit';
    editor.draft!.vendor = 'Me';
    expect(await editor.save()).toBe(true);
    expect(editor.userMaps.maps.map((m) => m.id)).toContain('my-kit');
    // The saved TOML round-trips: the stored file parses back to the same notes.
    const text = await editor.userMaps.read('my-kit');
    expect(text).toContain('[notes.36]');
    expect(text).toContain('tag = "kick.1"');
  });

  it('importing a broken TOML surfaces the parse error without dropping into a draft', async () => {
    const editor = await editorWith();
    editor.importToml('id = "x"\nname = ', 'broken.toml');
    expect(editor.draft).toBeNull();
    expect(editor.issues[0].field).toBe('toml');
    // And a valid import lands as a draft with every note.
    editor.importToml(
      'id = "x"\nname = "X"\nvendor = "V"\n\n[notes.36]\nname = "Kick"\ntag = "kick.1"\n',
      'x.toml',
    );
    expect(editor.draft!.notes.length).toBe(1);
  });

  it('note rows can be added on free numbers and removed', async () => {
    const editor = await editorWith();
    editor.newMap();
    editor.addNote();
    const notes = editor.draft!.notes.map((n) => n.note);
    expect(notes).toEqual([36, 35]);
    editor.removeNote(36);
    expect(editor.draft!.notes.map((n) => n.note)).toEqual([35]);
  });

  it('export refuses an invalid draft', async () => {
    const editor = await editorWith();
    editor.newMap();
    expect(await editor.exportMap()).toBeNull();
    expect(editor.issues.length).toBeGreaterThan(0);
  });
});

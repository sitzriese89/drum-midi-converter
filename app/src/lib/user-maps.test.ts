// The user maps store (SPEC §3.4): list/read/write/delete over an injected storage, the
// web fallback that refuses everything, and the user maps landing in the converter's
// dropdowns (App merges them into `loadCatalog`).
import { describe, expect, it } from 'vitest';
import { Converter } from './converter.svelte';
import { unavailableMapStorage, UserMaps, type MapStorage, mapName } from './user-maps.svelte';

const MY_KIT = `id = "my-kit"\nname = "My Kit"\nvendor = "Me"\n\n[notes.36]\nname = "Kick"\ntag = "kick.1"\n`;

/** An in-memory stand-in for the Tauri app-data storage. */
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
    exportFile: async () => {
      throw new Error('not implemented in the fake');
    },
  };
}

describe('UserMaps', () => {
  it('round-trips save, list, read and delete through the storage', async () => {
    const userMaps = new UserMaps(memoryStorage());
    await userMaps.save('my-kit', MY_KIT);
    expect(userMaps.maps).toEqual([{ id: 'my-kit', name: 'My Kit' }]);
    expect(await userMaps.read('my-kit')).toBe(MY_KIT);
    await userMaps.delete('my-kit');
    expect(userMaps.maps).toEqual([]);
    expect(await userMaps.read('my-kit')).toBeNull();
    expect(userMaps.error).toBeNull();
  });

  it('reads the display name from the TOML and falls back to the id', () => {
    expect(mapName(MY_KIT, 'my-kit')).toBe('My Kit');
    expect(mapName('id = "x"\n', 'x')).toBe('x');
    expect(mapName('name = "a\\"b"\n', 'x')).toBe('a"b');
  });

  it('reports write failures instead of throwing', async () => {
    const storage = memoryStorage();
    storage.write = async () => {
      throw new Error('disk full');
    };
    const userMaps = new UserMaps(storage);
    await userMaps.save('my-kit', MY_KIT);
    expect(userMaps.error).toContain('disk full');
    expect(userMaps.maps).toEqual([]);
  });

  it('the web fallback is unavailable and refuses every operation', async () => {
    const userMaps = new UserMaps(unavailableMapStorage);
    expect(userMaps.available).toBe(false);
    await expect(userMaps.read('my-kit')).rejects.toThrow(/desktop app/);
    // save reports the failure instead of rejecting: the UI shows the error line.
    await userMaps.save('my-kit', MY_KIT);
    expect(userMaps.error).toContain('desktop app');
    await userMaps.load();
    expect(userMaps.maps).toEqual([]);
  });

  it('user maps appear in the converter dropdowns after saving', async () => {
    const userMaps = new UserMaps(memoryStorage());
    await userMaps.save('my-kit', MY_KIT);
    const converter = new Converter();
    await converter.loadCatalog(userMaps.maps);
    expect(converter.maps.map((m) => m.id)).toContain('my-kit');
    // The built-ins are still there, first.
    expect(converter.maps[0].id).toBe('gm');
  });
});

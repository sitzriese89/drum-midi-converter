// The user's own drum maps (SPEC §3.4, Q6), in the same TOML schema as the built-in maps.
// Under Tauri they are `<id>.toml` files in the OS app-data folder, written and read through
// the Rust commands (ADR-002). In the web build (M4) they live in the browser's
// localStorage, one entry per map; when the browser blocks storage they are unavailable
// and the editor says so instead of pretending to save.
//
// Like the conversion flow, the list lives in this class: every change bumps `revision`,
// and `App` re-merges the user maps into the source/target dropdowns when it changes.

import { invoke } from '@tauri-apps/api/core';
import { downloadFile, isTauri } from './files';
import { parseMapToml } from './drum-map';

/** Where user maps are kept: the Tauri app-data commands, browser storage, or a test double. */
export interface MapStorage {
  /** False when user maps can't be persisted here (browser storage blocked). */
  readonly available: boolean;
  /** A caveat about where maps are kept, shown in the editor (e.g. "this browser only"). */
  readonly notice?: string;
  list(): Promise<string[]>;
  read(id: string): Promise<string | null>;
  write(id: string, content: string): Promise<void>;
  delete(id: string): Promise<void>;
  /** Asks where to save a shared map TOML. Null when cancelled. */
  exportFile(id: string, content: string): Promise<string | null>;
}

/** The Tauri implementation: all access goes through Rust commands. */
export const tauriMapStorage: MapStorage = {
  available: true,
  list: () => invoke<string[]>('list_user_maps'),
  read: (id) => invoke<string | null>('read_map', { id }),
  write: (id, content) => invoke<void>('write_map', { id, content }),
  delete: (id) => invoke<void>('delete_map', { id }),
  exportFile: (id, content) => invoke<string | null>('export_map_file', { id, content }),
};

/** Prefix of the localStorage keys that hold user maps: `<prefix><id>` → TOML text. */
export const BROWSER_KEY_PREFIX = 'drum-midi-converter.map.';

/** The web implementation over a `Storage` (localStorage in the app, a fake in tests).
 * Ids aren't paths here, but `DrumMap::build` has already checked them before any save. */
export function browserMapStorage(storage: Storage): MapStorage {
  return {
    available: true,
    notice:
      'Maps are saved in this browser only. Clearing site data deletes them; export a map to keep a copy.',
    list: async () => {
      const ids: string[] = [];
      for (let i = 0; i < storage.length; i++) {
        const key = storage.key(i);
        if (key?.startsWith(BROWSER_KEY_PREFIX)) ids.push(key.slice(BROWSER_KEY_PREFIX.length));
      }
      return ids.sort();
    },
    read: async (id) => storage.getItem(BROWSER_KEY_PREFIX + id),
    write: async (id, content) => {
      try {
        storage.setItem(BROWSER_KEY_PREFIX + id, content);
      } catch (error) {
        // QuotaExceededError and friends: say what failed, not just the DOM error name.
        throw new Error(`could not save map "${id}" in browser storage: ${message(error)}`, {
          cause: error,
        });
      }
    },
    delete: async (id) => storage.removeItem(BROWSER_KEY_PREFIX + id),
    exportFile: async (id, content) => {
      const name = `${id}.toml`;
      downloadFile(name, content, 'application/toml');
      return name;
    },
  };
}

/** The app's localStorage, or null when the browser blocks it (privacy settings, sandboxed
 * frames): reading the property itself can throw. */
function localStorageOrNull(): Storage | null {
  try {
    const storage = globalThis.localStorage;
    if (!storage) return null;
    const probe = `${BROWSER_KEY_PREFIX}__probe__`;
    storage.setItem(probe, '');
    storage.removeItem(probe);
    return storage;
  } catch {
    return null;
  }
}

/** The fallback when nothing can persist user maps: unavailable, and everything says so. */
export const unavailableMapStorage: MapStorage = {
  available: false,
  list: async () => [],
  read: async () => {
    throw new Error(unavailableMessage);
  },
  write: async () => {
    throw new Error(unavailableMessage);
  },
  delete: async () => {
    throw new Error(unavailableMessage);
  },
  exportFile: async () => {
    throw new Error(unavailableMessage);
  },
};

const unavailableMessage =
  'user maps need browser storage, which is blocked here (or use the desktop app)';

/** The storage for this runtime: Tauri commands, browser storage on the web, or the
 * unavailable fallback when the browser blocks storage. */
export function mapStorage(): MapStorage {
  if (isTauri()) return tauriMapStorage;
  const storage = localStorageOrNull();
  return storage ? browserMapStorage(storage) : unavailableMapStorage;
}

/** The display name of a stored map, read through the core's TOML parser
 * (SPEC §2.2: the core is the one schema reader — no hand-rolled regex here).
 * Falls back to the id only when the file doesn't parse (e.g. truncated or
 * hand-edited), so the list still shows every stored id. */
export function mapName(toml: string, id: string): string {
  return parseMapToml(toml).map?.name ?? id;
}

export class UserMaps {
  /** Bumped after every change to the stored maps; read it to re-read `maps`. */
  revision = $state(0);
  /** All stored user maps as catalog entries, sorted by id. */
  maps = $state<{ id: string; name: string }[]>([]);
  /** The last problem from loading, saving or deleting, for the UI's error line. */
  error = $state<string | null>(null);

  constructor(private readonly storage: MapStorage = mapStorage()) {}

  /** True when user maps can be persisted in this runtime. */
  get available(): boolean {
    return this.storage.available;
  }

  /** The storage's caveat about where maps are kept, if any. */
  get notice(): string | null {
    return this.storage.notice ?? null;
  }

  /** Reads the stored map ids and their display names. Never rejects. */
  async load(): Promise<void> {
    this.error = null;
    try {
      const ids = await this.storage.list();
      this.maps = await Promise.all(
        ids.map(async (id) => ({ id, name: mapName((await this.storage.read(id)) ?? '', id) })),
      );
    } catch (error) {
      this.maps = [];
      this.error = message(error);
    }
    this.revision += 1;
  }

  /** Reads a stored map's TOML text; null when it doesn't exist. */
  async read(id: string): Promise<string | null> {
    return this.storage.read(id);
  }

  /** Writes a validated map's TOML and refreshes the list. Rejects on failure. */
  async save(id: string, toml: string): Promise<void> {
    this.error = null;
    try {
      await this.storage.write(id, toml);
      await this.load();
    } catch (error) {
      this.error = message(error);
    }
  }

  /** Deletes a stored map and refreshes the list. */
  async delete(id: string): Promise<void> {
    this.error = null;
    try {
      await this.storage.delete(id);
      await this.load();
    } catch (error) {
      this.error = message(error);
    }
  }

  /** Exports a map TOML through a save dialog (desktop) or a browser download (web). */
  async exportFile(id: string, toml: string): Promise<string | null> {
    this.error = null;
    try {
      return await this.storage.exportFile(id, toml);
    } catch (error) {
      this.error = message(error);
      return null;
    }
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

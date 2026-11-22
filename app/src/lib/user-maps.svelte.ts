// The user's own drum maps (SPEC §3.4, Q6): `<id>.toml` files in the OS app-data folder,
// in the same schema as the built-in maps, written and read through the Rust commands
// (ADR-002). Under Tauri they live on disk; on the web they are unavailable — the editor
// surfaces that instead of pretending to save (M4 may add IndexedDB/localStorage).
//
// Like the conversion flow, the list lives in this class: every change bumps `revision`,
// and `App` re-merges the user maps into the source/target dropdowns when it changes.

import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './files';
import { parseMapToml } from './drum-map';

/** Where user maps are kept: the Tauri app-data commands, or a test double. */
export interface MapStorage {
  /** False when user maps can't be persisted here (the plain web build). */
  readonly available: boolean;
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

/** The web fallback: user maps are unavailable, and everything says so. */
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
  'user maps need the desktop app (they are stored in the app-data folder)';

/** The storage for this runtime: Tauri commands, or the unavailable fallback on the web. */
export function mapStorage(): MapStorage {
  return isTauri() ? tauriMapStorage : unavailableMapStorage;
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

  /** Exports a map TOML through a save dialog (desktop) or a download (not on the web). */
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

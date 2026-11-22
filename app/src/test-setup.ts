import '@testing-library/jest-dom/vitest';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { initDrumMap } from './lib/drum-map';

// jsdom can't fetch the .wasm file, so load the module from disk.
await initDrumMap(await readFile(join(import.meta.dirname, 'lib/wasm/drum_map_wasm_bg.wasm')));

// Filesystem layer (ADR-002): under Tauri, files come from Rust commands only; in the web
// build (M4) they come from a browser file input or drop, and leave as a browser download.

import { invoke } from '@tauri-apps/api/core';

/** A MIDI file read from disk or dropped/picked in the webview. */
export interface MidiFile {
  name: string;
  bytes: Uint8Array;
}

/** True when running inside the Tauri shell (vs a plain browser). */
export function isTauri(): boolean {
  return '__TAURI_INTERNALS__' in globalThis;
}

/** Opens a native open dialog (Rust-side) and reads the picked file. Null when cancelled.
 * Tauri only: the web build picks through a file input instead. */
export async function pickMidiFile(): Promise<MidiFile | null> {
  const picked = await invoke<{ name: string; bytes: number[] } | null>('pick_midi_file');
  return picked ? { name: picked.name, bytes: new Uint8Array(picked.bytes) } : null;
}

/** Reads a dropped or picked `File` (works in the Tauri webview and in browsers). */
export async function readMidiFile(file: File): Promise<MidiFile> {
  return { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) };
}

/** Saves under a native save dialog; null when cancelled. Web: triggers a browser download. */
export async function saveMidiFile(name: string, bytes: Uint8Array): Promise<string | null> {
  if (isTauri()) {
    return invoke<string | null>('save_midi_file', { name, bytes: Array.from(bytes) });
  }
  downloadFile(name, bytes, 'audio/midi');
  return name;
}

/** Starts a browser download of `content` as `name` (the web build's "save"). */
export function downloadFile(name: string, content: Uint8Array | string, type: string): void {
  const blob = new Blob([content as BlobPart], { type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  anchor.style.display = 'none';
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  // Revoking right after click() can cancel the download in some browsers (Firefox):
  // give it a moment to start.
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** Writes a temp file for drag-out and returns its `file://` URL. Tauri only: a browser
 * can't hand a DAW a real file, so the web build offers the download instead. */
export async function writeTempFile(name: string, bytes: Uint8Array): Promise<string> {
  const path = await invoke<string>('write_temp_file', { name, bytes: Array.from(bytes) });
  // URL-encode the path: a DAW/explorer drop target reads it back as UTF-8.
  return `file://${encodeURI(path)}`;
}

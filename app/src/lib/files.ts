// Filesystem layer (ADR-002): under Tauri, files come from Rust commands only; on the web it
// degrades to a file input and a browser download (M4 builds on this).

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

/** Opens a native open dialog (Rust-side) and reads the picked file. Null when cancelled. */
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
  const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: 'audio/midi' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  URL.revokeObjectURL(url);
  return name;
}

/** Writes a temp file for drag-out and returns its `file://` URL. Web: an object URL. */
export async function writeTempFile(name: string, bytes: Uint8Array): Promise<string> {
  if (isTauri()) {
    const path = await invoke<string>('write_temp_file', { name, bytes: Array.from(bytes) });
    // URL-encode the path: a DAW/explorer drop target reads it back as UTF-8.
    return `file://${encodeURI(path)}`;
  }
  return URL.createObjectURL(new Blob([bytes as BlobPart], { type: 'audio/midi' }));
}

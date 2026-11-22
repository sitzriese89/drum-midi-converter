//! Filesystem commands (ADR-002): read the input MIDI file through a native open dialog,
//! write the converted file through a native save dialog, and write a temp file for
//! drag-out. The dialogs open from Rust, so the webview never names a path itself.

use std::fs;

use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

/// A MIDI file the user picked in an open dialog.
#[derive(Serialize)]
pub struct PickedFile {
    /// File name only (no path) — the webview never learns the full path.
    pub name: String,
    /// Raw SMF bytes.
    pub bytes: Vec<u8>,
}

/// A MIDI-filtered file dialog builder: the filter both commands share.
fn midi_dialog<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    app.dialog().file().add_filter("MIDI", &["mid", "midi"])
}

/// Opens a native open dialog for `.mid`/`.midi` files and reads the picked file.
/// `None` when the dialog was cancelled.
#[tauri::command]
pub async fn pick_midi_file<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<PickedFile>, String> {
    // Async commands run off the main thread, where a blocking dialog is allowed.
    let Some(picked) = midi_dialog(&app).blocking_pick_file() else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|e| format!("unusable file path: {e}"))?;
    let bytes = fs::read(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(Some(PickedFile {
        name: file_name(&path),
        bytes,
    }))
}

/// Opens a native save dialog (suggested name `name`) and writes the converted SMF bytes
/// there. Returns the path written, or `None` when the dialog was cancelled.
#[tauri::command]
pub async fn save_midi_file<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    name: String,
    bytes: Vec<u8>,
) -> Result<Option<String>, String> {
    // Async commands run off the main thread, where a blocking dialog is allowed.
    let Some(picked) = midi_dialog(&app).set_file_name(&name).blocking_save_file() else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|e| format!("unusable file path: {e}"))?;
    write(&path, &bytes)?;
    Ok(Some(path.display().to_string()))
}

/// Writes a temp file for drag-out and returns its path: the webview turns it into a
/// `file://` URL in the drag payload, so a DAW/explorer can read the converted file.
#[tauri::command]
pub async fn write_temp_file(name: String, bytes: Vec<u8>) -> Result<String, String> {
    let dir = std::env::temp_dir().join("drum-midi-converter");
    fs::create_dir_all(&dir).map_err(|e| format!("could not create the temp folder: {e}"))?;
    let path = dir.join(sanitize(&name));
    write(&path, &bytes)?;
    Ok(path.display().to_string())
}

/// Writes bytes, mapping the error into the string Tauri commands return.
fn write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// The file name only, UTF-8-lossy, no path components.
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Strips path separators and reserved characters from a drag-out file name.
fn sanitize(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect()
}

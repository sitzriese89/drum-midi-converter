//! Drum MIDI Converter desktop shell.
//!
//! The UI calls the drum-map core directly as WASM; this process only hosts the window and
//! the filesystem commands (read the input file, write the output/temp file, ADR-002).

// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod files;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            files::pick_midi_file,
            files::save_midi_file,
            files::write_temp_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Drum MIDI Converter window");
}

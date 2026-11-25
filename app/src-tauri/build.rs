// Declaring the app's commands makes Tauri generate an `allow-<command>` permission for each,
// so a command is callable only when `capabilities/default.json` grants it.
// Keep in sync with `generate_handler!` in src/main.rs: a command missing here has no
// permission, and the capability that grants it fails the build on a clean checkout.
const COMMANDS: &[&str] = &[
    // Files (src/files.rs).
    "pick_midi_file",
    "save_midi_file",
    "write_temp_file",
    // User maps (src/storage.rs).
    "read_map",
    "write_map",
    "list_user_maps",
    "delete_map",
    "export_map_file",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}

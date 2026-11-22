//! User map persistence (SPEC §3.4, ADR-002): user-authored drum maps live in
//! the OS app-data directory as `<id>.toml`, in the same schema as the
//! built-ins. The webview never names a path: it passes a map id and Rust
//! builds the file path from it, so a name can never point outside the
//! folder. Writes are atomic — the content goes to a temporary file in the
//! same folder, which then replaces the map by rename, so a crash leaves
//! either the old or the new file, never half of one.
//!
//! The file logic is plain functions over a directory, tested without a
//! Tauri runtime; the `#[tauri::command]` wrappers at the bottom only resolve
//! the folder and map errors.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Longest accepted map id, in characters (the core's `DrumMap::build` rule).
pub const MAX_NAME_LEN: usize = 64;

/// Names Windows reserves for devices; `con.toml` can't be created there.
const RESERVED_ON_WINDOWS: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// What can go wrong with a stored map.
#[derive(Debug)]
pub enum StorageError {
    /// The id isn't a valid map id (see [`validate_name`]).
    InvalidName(String),
    /// The filesystem refused; the message says which map.
    Io { context: String, source: io::Error },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(
                f,
                "invalid map id {name:?}: use 1-{MAX_NAME_LEN} lowercase letters, digits or \
                 '-', starting with a letter or digit"
            ),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidName(_) => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

fn io_error(context: impl Into<String>) -> impl FnOnce(io::Error) -> StorageError {
    move |source| StorageError::Io {
        context: context.into(),
        source,
    }
}

/// Checks a map id: 1–64 characters from `a-z`, `0-9`, `-`, starting with a
/// letter or digit, and not a Windows device name. Lowercase only, so ids
/// can't collide on case-insensitive filesystems. The same rule is enforced
/// by the core's `DrumMap::build` (the id is also the file name).
pub fn validate_name(name: &str) -> Result<(), StorageError> {
    let mut chars = name.chars();
    let valid = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !RESERVED_ON_WINDOWS.contains(&name);
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidName(name.to_owned()))
    }
}

/// The file that holds map `id` in `dir`, after checking the id.
pub fn map_path(dir: &Path, id: &str) -> Result<PathBuf, StorageError> {
    validate_name(id)?;
    Ok(dir.join(format!("{id}.toml")))
}

/// Reads map `id`; `None` if it doesn't exist.
pub fn read_map(dir: &Path, id: &str) -> Result<Option<String>, StorageError> {
    let path = map_path(dir, id)?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(format!("could not read map {id:?}"))(e)),
    }
}

/// Writes map `id` atomically, creating `dir` if needed.
pub fn write_map(dir: &Path, id: &str, content: &str) -> Result<(), StorageError> {
    let path = map_path(dir, id)?;
    fs::create_dir_all(dir).map_err(io_error("could not create the app-data folder"))?;
    write_atomic(&path, content).map_err(io_error(format!("could not write map {id:?}")))
}

/// Ids of all stored user maps, sorted. Files that aren't valid map files
/// (temporary files, other extensions, invalid ids) are skipped. A missing
/// folder means no maps.
pub fn list_maps(dir: &Path) -> Result<Vec<String>, StorageError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error("could not list the app-data folder")(e)),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_error("could not list the app-data folder"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(id) = file_name.strip_suffix(".toml") else {
            continue;
        };
        if validate_name(id).is_ok() {
            ids.push(id.to_owned());
        }
    }
    ids.sort();
    Ok(ids)
}

/// Deletes map `id`. Deleting a map that doesn't exist is not an error.
pub fn delete_map(dir: &Path, id: &str) -> Result<(), StorageError> {
    let path = map_path(dir, id)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(format!("could not delete map {id:?}"))(e)),
    }
}

/// Writes `content` to `path` through a temporary file in the same folder,
/// flushed to disk, then renamed over `path` (which replaces an existing file
/// on every platform).
pub fn write_atomic(path: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("map");
    let tmp = dir.join(format!(
        ".{file_name}.{}-{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(content.as_ref())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        // Best effort: don't leave the temporary file behind.
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Tauri commands, called from `app/src/lib/user-maps.svelte.ts`.
pub mod commands {
    use std::path::PathBuf;

    use tauri::{AppHandle, Manager, Runtime};
    use tauri_plugin_dialog::DialogExt;

    use super::write_atomic;

    fn maps_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
        app.path()
            .app_data_dir()
            .map_err(|e| format!("could not find the app-data folder: {e}"))
    }

    #[tauri::command]
    pub fn read_map<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Option<String>, String> {
        super::read_map(&maps_dir(&app)?, &id).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn write_map<R: Runtime>(
        app: AppHandle<R>,
        id: String,
        content: String,
    ) -> Result<(), String> {
        super::write_map(&maps_dir(&app)?, &id, &content).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn list_user_maps<R: Runtime>(app: AppHandle<R>) -> Result<Vec<String>, String> {
        super::list_maps(&maps_dir(&app)?).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn delete_map<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
        super::delete_map(&maps_dir(&app)?, &id).map_err(|e| e.to_string())
    }

    /// Asks where to save a shared map TOML (suggested name `id`) and writes
    /// it there. `None` when the dialog was cancelled. Used for the editor's
    /// export; import goes through a webview file input, so the webview never
    /// names a read path either.
    #[tauri::command]
    pub async fn export_map_file<R: Runtime>(
        app: AppHandle<R>,
        id: String,
        content: String,
    ) -> Result<Option<String>, String> {
        // Async commands run off the main thread, where a blocking dialog is
        // allowed.
        let Some(picked) = app
            .dialog()
            .file()
            .add_filter("TOML", &["toml"])
            .set_file_name(format!("{id}.toml"))
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = picked
            .into_path()
            .map_err(|e| format!("unusable file path: {e}"))?;
        write_atomic(&path, content.into_bytes())
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        Ok(Some(path.display().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty folder under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "drum-midi-converter-storage-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn ids_follow_the_core_map_id_rule() {
        for id in ["gm", "my-kit", "kit2", &"a".repeat(MAX_NAME_LEN)] {
            assert!(validate_name(id).is_ok(), "{id:?} should be valid");
        }
        for id in [
            "",
            "Upper",
            "with space",
            "with.dot",
            "a/b",
            "../escape",
            "C:\\abs",
            "-lead",
            "con",
            "NUL",
        ] {
            assert!(validate_name(id).is_err(), "{id:?} should be invalid");
        }
        assert!(validate_name(&"a".repeat(MAX_NAME_LEN + 1)).is_err());
    }

    #[test]
    fn written_maps_read_back_unchanged() {
        let dir = TempDir::new();
        let content = "id = \"my-kit\"\nname = \"My Kit\"\n";
        write_map(&dir.0, "my-kit", content).unwrap();
        assert_eq!(
            read_map(&dir.0, "my-kit").unwrap().as_deref(),
            Some(content)
        );
        assert_eq!(
            fs::read_to_string(dir.0.join("my-kit.toml")).unwrap(),
            content
        );
    }

    #[test]
    fn writing_replaces_the_map_and_leaves_no_temporary_files() {
        let dir = TempDir::new();
        write_map(&dir.0, "my-kit", "a = 1\n").unwrap();
        write_map(&dir.0, "my-kit", "b = 2\n").unwrap();
        assert_eq!(
            read_map(&dir.0, "my-kit").unwrap().as_deref(),
            Some("b = 2\n")
        );
        let files: Vec<_> = fs::read_dir(&dir.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(files, vec!["my-kit.toml"]);
    }

    #[test]
    fn writing_creates_the_folder() {
        let dir = TempDir::new();
        let nested = dir.0.join("not").join("there");
        write_map(&nested, "my-kit", "x = 1\n").unwrap();
        assert_eq!(
            read_map(&nested, "my-kit").unwrap().as_deref(),
            Some("x = 1\n")
        );
    }

    #[test]
    fn reading_a_missing_map_gives_none() {
        let dir = TempDir::new();
        assert_eq!(read_map(&dir.0, "my-kit").unwrap(), None);
    }

    #[test]
    fn list_returns_sorted_map_ids_only() {
        let dir = TempDir::new();
        write_map(&dir.0, "my-kit", "").unwrap();
        write_map(&dir.0, "another-kit", "").unwrap();
        fs::write(dir.0.join("notes.txt"), "not a map").unwrap();
        fs::write(dir.0.join("Upper.toml"), "invalid id").unwrap();
        fs::write(dir.0.join(".my-kit.toml.1-0.tmp"), "temporary").unwrap();
        fs::create_dir(dir.0.join("folder.toml")).unwrap();
        assert_eq!(list_maps(&dir.0).unwrap(), vec!["another-kit", "my-kit"]);
    }

    #[test]
    fn list_of_a_missing_folder_is_empty() {
        let dir = TempDir::new();
        assert!(list_maps(&dir.0.join("missing")).unwrap().is_empty());
    }

    #[test]
    fn delete_removes_the_map_and_ignores_missing_ones() {
        let dir = TempDir::new();
        write_map(&dir.0, "my-kit", "x = 1\n").unwrap();
        delete_map(&dir.0, "my-kit").unwrap();
        assert_eq!(read_map(&dir.0, "my-kit").unwrap(), None);
        delete_map(&dir.0, "my-kit").unwrap();
    }

    #[test]
    fn ids_that_could_escape_the_folder_are_rejected_everywhere() {
        let dir = TempDir::new();
        let outside = dir.0.join("outside");
        fs::create_dir(&outside).unwrap();
        let maps = outside.join("maps");
        for id in [
            "../escape",
            "..\\escape",
            "/abs",
            "C:\\abs",
            "a/b",
            "..",
            "",
        ] {
            assert!(matches!(
                read_map(&maps, id),
                Err(StorageError::InvalidName(_))
            ));
            assert!(matches!(
                write_map(&maps, id, "x"),
                Err(StorageError::InvalidName(_))
            ));
            assert!(matches!(
                delete_map(&maps, id),
                Err(StorageError::InvalidName(_))
            ));
        }
        assert!(!maps.exists(), "a rejected write must not create anything");
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }

    #[test]
    fn errors_name_the_map() {
        let err = validate_name("../x").unwrap_err().to_string();
        assert!(err.contains("\"../x\""), "{err}");
    }
}

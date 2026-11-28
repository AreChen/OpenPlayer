use std::path::Path;

use super::{collect, startup::StartupMediaState};

#[tauri::command]
pub fn media_files_in_directory(path: String) -> Result<Vec<String>, String> {
    collect::collect_media_files_in_directory(Path::new(&path))
}

#[tauri::command]
pub fn media_files_from_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    collect::collect_media_files_from_paths(&paths)
}

#[tauri::command]
pub fn startup_media_paths(state: tauri::State<'_, StartupMediaState>) -> Vec<String> {
    state.paths()
}

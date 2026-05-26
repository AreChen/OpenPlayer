use std::path::{Path, PathBuf};

pub(crate) fn window_reveal_path(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("file path is empty".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.exists() {
        return Err(format!("file path does not exist: {}", path.display()));
    }

    reveal_path_in_file_manager(&path)
}

pub(crate) fn window_open_directory(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("directory path is empty".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_dir() {
        return Err(format!("directory path does not exist: {}", path.display()));
    }

    open_directory_in_file_manager(&path)
}

#[cfg(windows)]
fn open_directory_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open directory: {error}"))
}

#[cfg(target_os = "macos")]
fn open_directory_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open directory: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_directory_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open directory: {error}"))
}

#[cfg(windows)]
fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open file location: {error}"))
}

#[cfg(target_os = "macos")]
fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open file location: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    let target = path
        .parent()
        .filter(|parent| parent.exists())
        .unwrap_or(path);
    std::process::Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open file location: {error}"))
}

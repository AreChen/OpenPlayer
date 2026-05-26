use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::{extensions::is_supported_media_path, sort::sort_media_paths};

pub(in crate::media_paths) fn collect_media_files_in_directory(
    directory: &Path,
) -> Result<Vec<String>, String> {
    if !directory.is_dir() {
        return Err("selected path is not a directory".to_string());
    }

    let mut paths = Vec::new();
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("failed to read media directory: {error}"))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("failed to read media directory entry: {error}"))?;
        let path = entry.path();
        if path.is_file() && is_supported_media_path(&path) {
            paths.push(path);
        }
    }
    sort_media_paths(&mut paths);

    Ok(paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}

pub(in crate::media_paths) fn collect_media_files_from_paths(
    paths: &[String],
) -> Result<Vec<String>, String> {
    let mut media_paths = Vec::new();
    for raw_path in paths {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            continue;
        }

        let path = PathBuf::from(trimmed);
        if path.is_dir() {
            media_paths.extend(
                collect_media_files_in_directory(&path)?
                    .into_iter()
                    .map(PathBuf::from),
            );
        } else if path.is_file() && is_supported_media_path(&path) {
            media_paths.push(path);
        }
    }

    sort_media_paths(&mut media_paths);
    let mut seen = HashSet::new();
    let unique_paths = media_paths
        .into_iter()
        .filter_map(|path| {
            let text = path.to_string_lossy().to_string();
            if seen.insert(text.to_ascii_lowercase()) {
                Some(text)
            } else {
                None
            }
        })
        .collect();

    Ok(unique_paths)
}

use super::super::*;

pub(in crate::mpv_embed) fn capture_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create capture directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().picture_dir() {
        directory.push("OpenPlayer");
        directory.push("Captures");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve capture directory: {error}"))?;
    directory.push("captures");
    Ok(directory)
}

pub(in crate::mpv_embed) fn recording_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create recording directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().video_dir() {
        directory.push("OpenPlayer");
        directory.push("Recordings");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve recording directory: {error}"))?;
    directory.push("recordings");
    Ok(directory)
}

pub(in crate::mpv_embed) fn normalize_capture_directory_override(
    directory: Option<String>,
) -> Result<Option<PathBuf>, String> {
    let Some(directory) = directory
        .as_deref()
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
    else {
        return Ok(None);
    };
    if directory.len() > 1024 {
        return Err("capture directory path is too long".to_string());
    }
    let path = PathBuf::from(directory);
    if !path.is_absolute() {
        return Err("capture directory path must be absolute".to_string());
    }
    if path.is_file() {
        return Err("capture directory path is not a directory".to_string());
    }
    Ok(Some(path))
}

use std::path::Path;

use redb::ReadableTable;

use super::{super::*, time::now_millis};

pub(in crate::playback_store) fn normalize_update(
    update: PlaybackHistoryUpdate,
) -> Result<PlaybackHistoryEntry, String> {
    let path = update.path.trim().to_string();
    if path.is_empty() {
        return Err("playback history path is empty".to_string());
    }

    Ok(PlaybackHistoryEntry {
        name: update
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| media_name_from_path(&path)),
        path,
        position: normalize_non_negative_number(update.position),
        duration: normalize_non_negative_number(update.duration),
        updated_at: update.updated_at.unwrap_or_else(now_millis).max(0),
    })
}

pub(in crate::playback_store) fn normalize_non_negative_number(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

pub(in crate::playback_store) fn resume_position_for_entry(position: f64, duration: f64) -> f64 {
    if !position.is_finite() || !duration.is_finite() || duration <= 0.0 || position <= 0.0 {
        return 0.0;
    }

    let clamped = position.clamp(0.0, duration);
    let ratio = clamped / duration;
    if !(MIN_RESUME_PROGRESS_RATIO..RESUME_END_PROGRESS_RATIO).contains(&ratio) {
        0.0
    } else {
        clamped
    }
}

pub(in crate::playback_store) fn updated_index_key(updated_at: i64, path: &str) -> String {
    let newest_first = u64::MAX - updated_at.max(0) as u64;
    format!("{newest_first:020}|{path}")
}

pub(in crate::playback_store) fn store_key_for_path(path: &str) -> String {
    let trimmed = path.trim();
    let mut normalized = trimmed.replace('/', "\\");
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("\\\\?\\unc\\") {
        normalized = format!("\\\\{}", &normalized[8..]);
    } else if lower.starts_with("\\\\?\\") {
        normalized = normalized[4..].to_string();
    }

    if is_windows_drive_path(&normalized) || normalized.starts_with("\\\\") {
        normalized.to_lowercase()
    } else {
        trimmed.to_string()
    }
}

pub(in crate::playback_store) fn is_windows_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\'
}

pub(in crate::playback_store) fn get_by_normalized_or_legacy_key<'a, T>(
    table: &'a T,
    path: &str,
) -> Result<Option<redb::AccessGuard<'a, &'static str>>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let key = store_key_for_path(path);
    if let Some(stored) = table
        .get(key.as_str())
        .map_err(|error| format!("failed to read playback store entry: {error}"))?
    {
        return Ok(Some(stored));
    }

    let legacy_key = path.trim();
    if legacy_key == key {
        return Ok(None);
    }

    table
        .get(legacy_key)
        .map_err(|error| format!("failed to read playback store entry: {error}"))
}

pub(in crate::playback_store) fn media_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

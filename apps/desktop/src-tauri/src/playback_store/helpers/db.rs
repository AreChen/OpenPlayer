use std::{path::Path, thread, time::Duration};

use redb::{Database, ReadableTable, ReadableTableMetadata};

use super::super::*;

pub(in crate::playback_store) fn create_database_with_retry(
    path: &Path,
    label: &str,
) -> Result<Database, String> {
    let mut last_error = None;
    for _ in 0..16 {
        match Database::create(path) {
            Ok(database) => return Ok(database),
            Err(error) => {
                last_error = Some(error.to_string());
                thread::sleep(Duration::from_millis(20));
            }
        }
    }

    Err(format!(
        "failed to open {label} database: {}",
        last_error.unwrap_or_else(|| "unknown redb error".to_string())
    ))
}

pub(in crate::playback_store) fn table_keys<T>(
    table: &T,
    label: &str,
) -> Result<Vec<String>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    table
        .iter()
        .map_err(|error| format!("failed to scan {label}: {error}"))?
        .map(|item| {
            item.map(|(key, _)| key.value().to_string())
                .map_err(|error| format!("failed to read {label}: {error}"))
        })
        .collect()
}

pub(in crate::playback_store) fn prune_history(
    by_path: &mut redb::Table<'_, &str, &str>,
    by_updated: &mut redb::Table<'_, &str, &str>,
) -> Result<(), String> {
    while by_updated
        .len()
        .map_err(|error| format!("failed to count playback history entries: {error}"))?
        > HISTORY_LIMIT as u64
    {
        let oldest = by_updated
            .last()
            .map_err(|error| format!("failed to find old playback history entry: {error}"))?
            .map(|(index_key, path)| (index_key.value().to_string(), path.value().to_string()));
        let Some((index_key, path)) = oldest else {
            break;
        };
        by_updated
            .remove(index_key.as_str())
            .map_err(|error| format!("failed to prune playback history index: {error}"))?;
        by_path
            .remove(path.as_str())
            .map_err(|error| format!("failed to prune playback history entry: {error}"))?;
    }

    Ok(())
}

pub(in crate::playback_store) fn prune_network_stream_history(
    by_url: &mut redb::Table<'_, &str, &str>,
    by_updated: &mut redb::Table<'_, &str, &str>,
) -> Result<(), String> {
    while by_updated
        .len()
        .map_err(|error| format!("failed to count network stream history entries: {error}"))?
        > NETWORK_STREAM_HISTORY_LIMIT as u64
    {
        let oldest = by_updated
            .last()
            .map_err(|error| format!("failed to find old network stream history entry: {error}"))?
            .map(|(index_key, url)| (index_key.value().to_string(), url.value().to_string()));
        let Some((index_key, url)) = oldest else {
            break;
        };
        by_updated
            .remove(index_key.as_str())
            .map_err(|error| format!("failed to prune network stream history index: {error}"))?;
        by_url
            .remove(url.as_str())
            .map_err(|error| format!("failed to prune network stream history entry: {error}"))?;
    }

    Ok(())
}

pub(in crate::playback_store) fn existing_history_keys(
    by_path: &redb::Table<'_, &str, &str>,
    normalized_key: &str,
    display_path: &str,
) -> Result<Vec<String>, String> {
    let mut keys = Vec::new();
    if by_path
        .get(normalized_key)
        .map_err(|error| format!("failed to read old playback history entry: {error}"))?
        .is_some()
    {
        keys.push(normalized_key.to_string());
    }

    let legacy_key = display_path.trim();
    if legacy_key != normalized_key
        && by_path
            .get(legacy_key)
            .map_err(|error| format!("failed to read old playback history entry: {error}"))?
            .is_some()
    {
        keys.push(legacy_key.to_string());
    }

    Ok(keys)
}

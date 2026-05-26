use redb::ReadableTable;

use super::codecs::decode_stored_theme_manifest;

pub(in crate::appearance_store) fn theme_manifests_for_plugin(
    table: &redb::Table<'_, &str, &str>,
    plugin_id: &str,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for item in table
        .iter()
        .map_err(|error| format!("failed to scan theme manifests: {error}"))?
    {
        let (id, value) =
            item.map_err(|error| format!("failed to read theme manifest: {error}"))?;
        let stored = decode_stored_theme_manifest(value.value())?;
        if stored.plugin_id == plugin_id {
            ids.push(id.value().to_string());
        }
    }
    Ok(ids)
}

pub(in crate::appearance_store) fn theme_belongs_to_plugin<T>(
    table: &T,
    theme_id: &str,
    plugin_id: &str,
) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let Some(stored) = table
        .get(theme_id)
        .map_err(|error| format!("failed to read active theme manifest: {error}"))?
    else {
        return Ok(false);
    };
    Ok(decode_stored_theme_manifest(stored.value())?.plugin_id == plugin_id)
}

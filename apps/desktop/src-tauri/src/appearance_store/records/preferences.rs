use redb::ReadableTable;

use crate::appearance_store::LANGUAGE_MODE_KEY;

pub(in crate::appearance_store) fn read_bool_setting<T>(
    table: &T,
    key: &str,
) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    Ok(table
        .get(key)
        .map_err(|error| format!("failed to read boolean setting {key}: {error}"))?
        .map(|value| value.value() == "true")
        .unwrap_or(false))
}

pub(in crate::appearance_store) fn read_language_mode_setting<T>(
    table: &T,
) -> Result<String, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let Some(value) = table
        .get(LANGUAGE_MODE_KEY)
        .map_err(|error| format!("failed to read language preference: {error}"))?
    else {
        return Ok("system".to_string());
    };

    validate_language_mode(value.value()).map(ToOwned::to_owned)
}

pub(in crate::appearance_store) fn validate_language_mode(
    mode: &str,
) -> Result<&'static str, String> {
    match mode {
        "system" => Ok("system"),
        "en-US" => Ok("en-US"),
        "zh-CN" => Ok("zh-CN"),
        _ => Err("invalid language mode".to_string()),
    }
}

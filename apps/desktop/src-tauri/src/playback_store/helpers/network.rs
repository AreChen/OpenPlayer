use super::{super::*, time::now_millis};

pub(in crate::playback_store) fn normalize_network_stream_update(
    update: NetworkStreamHistoryUpdate,
) -> Result<NetworkStreamHistoryEntry, String> {
    let (url, scheme) = normalize_network_stream_url(&update.url)?;
    Ok(NetworkStreamHistoryEntry {
        name: update
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| network_stream_name_from_url(&url)),
        url,
        scheme,
        updated_at: update.updated_at.unwrap_or_else(now_millis).max(0),
    })
}

pub(in crate::playback_store) fn normalize_network_stream_url(
    url: &str,
) -> Result<(String, String), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err("network stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err("network stream url must include a protocol".to_string());
    };
    let scheme = scheme.to_ascii_lowercase();
    if !is_supported_network_stream_scheme(&scheme) {
        return Err(format!("unsupported network stream protocol: {scheme}"));
    }
    if rest.trim_matches('/').is_empty() {
        return Err("network stream url must include a host or path".to_string());
    }
    Ok((format!("{scheme}://{rest}"), scheme))
}

pub(in crate::playback_store) fn is_supported_network_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps"
    )
}

pub(in crate::playback_store) fn network_stream_key_for_url(url: &str) -> String {
    url.trim().to_string()
}

pub(in crate::playback_store) fn network_stream_name_from_url(url: &str) -> String {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    if let Some(tail) = without_query
        .rsplit('/')
        .find(|part| !part.is_empty() && !part.contains("://"))
    {
        return tail.to_string();
    }
    let without_scheme = without_query
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(without_query);
    without_scheme
        .split('/')
        .next()
        .filter(|host| !host.is_empty())
        .unwrap_or(url)
        .to_string()
}

use super::super::primitives::validate_non_empty;

pub(super) fn validate_plugin_stream_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    validate_non_empty("plugin stream url", trimmed)?;
    if trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err("plugin stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err("plugin stream url must include a protocol".to_string());
    };
    if rest.trim_matches('/').is_empty() {
        return Err("plugin stream url must include a host or path".to_string());
    }
    if is_supported_plugin_stream_scheme(&scheme.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(format!("unsupported plugin stream protocol: {scheme}"))
    }
}

fn is_supported_plugin_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

use super::*;

pub(in crate::mpv_embed) fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("enter a local media path for mpv embed playback".to_string());
    }

    if trimmed.contains("://") {
        validate_media_stream_url(trimmed)?;
        return Ok(PathBuf::from(trimmed));
    }

    let path = PathBuf::from(trimmed);
    if !path.is_file() {
        return Err(format!("media path does not exist: {}", path.display()));
    }

    Ok(path)
}

pub(super) fn validate_media_stream_url(url: &str) -> Result<(), String> {
    if url.len() > 2048 || url.chars().any(char::is_whitespace) {
        return Err("media stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return Err("media stream url must include a protocol".to_string());
    };
    if rest.trim_matches('/').is_empty() {
        return Err("media stream url must include a host or path".to_string());
    }
    if is_supported_media_stream_scheme(&scheme.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(format!("unsupported media stream protocol: {scheme}"))
    }
}

fn is_supported_media_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

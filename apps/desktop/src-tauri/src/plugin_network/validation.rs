use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

const MAX_PLUGIN_NETWORK_BODY_FILE_BYTES: u64 = 25 * 1024 * 1024;

pub(super) fn plugin_network_request_url(value: &str) -> Result<reqwest::Url, String> {
    if value.is_empty() || value.len() > 2048 || value.chars().any(char::is_whitespace) {
        return Err("network.request requires an http or https url".to_string());
    }
    let url = reqwest::Url::parse(value)
        .map_err(|_| "network.request requires an http or https url".to_string())?;
    if matches!(url.scheme(), "http" | "https") {
        Ok(url)
    } else {
        Err("network.request requires an http or https url".to_string())
    }
}

pub(super) fn plugin_network_request_method(
    value: Option<&str>,
) -> Result<reqwest::Method, String> {
    let method = value.unwrap_or("GET").trim().to_uppercase();
    match method.as_str() {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS" => {
            reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| format!("network.request method is unsupported: {method}"))
        }
        _ => Err(format!("network.request method is unsupported: {method}")),
    }
}

pub(super) fn plugin_network_headers(
    value: Option<HashMap<String, String>>,
) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    let Some(value) = value else {
        return Ok(headers);
    };
    for (raw_key, raw_value) in value.into_iter().take(32) {
        let key = raw_key.trim();
        if key.is_empty() || key.len() > 64 {
            continue;
        }
        if raw_value.is_empty() || raw_value.len() > 1024 || raw_value.contains(['\r', '\n']) {
            continue;
        }
        let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) else {
            continue;
        };
        let Ok(header_value) = reqwest::header::HeaderValue::from_str(raw_value.trim()) else {
            continue;
        };
        headers.insert(name, header_value);
    }
    Ok(headers)
}

pub(super) fn plugin_network_body_file_path(
    app_data_dir: &Path,
    plugin_id: &str,
    path: &str,
) -> Result<PathBuf, String> {
    let plugin_id = validate_plugin_network_body_file_plugin_id(plugin_id)?;
    if path.trim().is_empty() || path.len() > 4096 {
        return Err("network.request bodyFile path is required".to_string());
    }
    let candidate = PathBuf::from(path);
    if !candidate.is_absolute() {
        return Err("network.request bodyFile path must be absolute".to_string());
    }
    let allowed_directory = app_data_dir.join("audio-clips").join(plugin_id);
    let allowed_directory = std::fs::canonicalize(&allowed_directory)
        .map_err(|_| "network.request bodyFile must be a managed plugin artifact".to_string())?;
    let candidate = std::fs::canonicalize(candidate)
        .map_err(|_| "network.request bodyFile must be a managed plugin artifact".to_string())?;
    if !candidate.starts_with(&allowed_directory) {
        return Err("network.request bodyFile must be a managed plugin artifact".to_string());
    }
    let metadata = std::fs::metadata(&candidate)
        .map_err(|_| "network.request bodyFile must be a readable file".to_string())?;
    if !metadata.is_file() {
        return Err("network.request bodyFile must be a readable file".to_string());
    }
    if metadata.len() == 0 {
        return Err("network.request bodyFile must not be empty".to_string());
    }
    if metadata.len() > MAX_PLUGIN_NETWORK_BODY_FILE_BYTES {
        return Err("network.request bodyFile is too large".to_string());
    }
    Ok(candidate)
}

pub(super) fn plugin_network_body_file_content_type(
    value: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 128 || value.contains(['\r', '\n']) || !value.contains('/') {
        return Err("network.request bodyFile contentType is invalid".to_string());
    }
    Ok(Some(value.to_string()))
}

fn validate_plugin_network_body_file_plugin_id(plugin_id: &str) -> Result<&str, String> {
    let plugin_id = plugin_id.trim();
    if plugin_id.len() > 128
        || !plugin_id.contains('.')
        || plugin_id.split('.').any(|segment| {
            segment.is_empty()
                || !segment
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_lowercase())
                || !segment.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
        })
    {
        return Err("network.request bodyFile invalid plugin id".to_string());
    }
    Ok(plugin_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_network_request_accepts_http_urls_only() {
        assert!(plugin_network_request_url("http://127.0.0.1:9243/status").is_ok());
        assert!(plugin_network_request_url("https://example.test/api").is_ok());
        assert!(plugin_network_request_url("rtsp://127.0.0.1:8554/webm_rtsp_1").is_err());
        assert!(plugin_network_request_url("http://127.0.0.1:9243/status with-space").is_err());
    }

    #[test]
    fn plugin_network_request_accepts_expected_methods_only() {
        assert_eq!(
            plugin_network_request_method(Some("get")).unwrap(),
            reqwest::Method::GET
        );
        assert_eq!(
            plugin_network_request_method(Some("OPTIONS")).unwrap(),
            reqwest::Method::OPTIONS
        );
        assert!(plugin_network_request_method(Some("TRACE")).is_err());
    }

    #[test]
    fn plugin_network_body_file_accepts_current_plugin_audio_artifacts_only() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-network-body-file-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let artifact_directory = directory
            .join("audio-clips")
            .join("dev.openplayer.ai-transcript");
        std::fs::create_dir_all(&artifact_directory).expect("artifact directory should be created");
        let artifact = artifact_directory.join("clip.wav");
        std::fs::write(&artifact, b"wav").expect("artifact should be written");

        assert_eq!(
            plugin_network_body_file_path(
                &directory,
                "dev.openplayer.ai-transcript",
                &artifact.to_string_lossy()
            )
            .expect("current plugin artifacts should be uploadable"),
            std::fs::canonicalize(&artifact).expect("artifact should canonicalize")
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn plugin_network_body_file_rejects_unmanaged_or_cross_plugin_paths() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-network-body-file-reject-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let current_directory = directory
            .join("audio-clips")
            .join("dev.openplayer.ai-transcript");
        let other_directory = directory.join("audio-clips").join("dev.openplayer.other");
        std::fs::create_dir_all(&current_directory)
            .expect("current artifact directory should be created");
        std::fs::create_dir_all(&other_directory)
            .expect("other artifact directory should be created");
        let current_artifact = current_directory.join("clip.wav");
        let other_artifact = other_directory.join("clip.wav");
        let unmanaged = directory.join("unmanaged.wav");
        std::fs::write(&current_artifact, b"wav").expect("current artifact should be written");
        std::fs::write(&other_artifact, b"wav").expect("other artifact should be written");
        std::fs::write(&unmanaged, b"wav").expect("unmanaged file should be written");

        assert!(
            plugin_network_body_file_path(
                &directory,
                "dev.openplayer.ai-transcript",
                &other_artifact.to_string_lossy()
            )
            .expect_err("plugins must not upload another plugin artifact")
            .contains("managed plugin artifact")
        );
        assert!(
            plugin_network_body_file_path(
                &directory,
                "dev.openplayer.ai-transcript",
                &unmanaged.to_string_lossy()
            )
            .expect_err("plugins must not upload unmanaged files")
            .contains("managed plugin artifact")
        );
        assert!(
            plugin_network_body_file_path(
                &directory,
                "../plugin",
                &current_artifact.to_string_lossy()
            )
            .expect_err("invalid plugin ids should be rejected")
            .contains("invalid plugin id")
        );
        assert!(
            plugin_network_body_file_path(&directory, "dev.openplayer.ai-transcript", "")
                .expect_err("empty paths should be rejected")
                .contains("bodyFile path")
        );

        let _ = std::fs::remove_dir_all(&directory);
    }
}

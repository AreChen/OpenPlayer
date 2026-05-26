use std::collections::HashMap;

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
}

use std::{collections::HashMap, path::PathBuf, time::Duration};

use super::{
    types::{PluginNetworkRequestArgs, PluginNetworkResponse},
    validation::{
        plugin_network_body_file_content_type, plugin_network_body_file_path,
        plugin_network_headers, plugin_network_request_method, plugin_network_request_url,
    },
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

const MAX_PLUGIN_NETWORK_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_PLUGIN_NETWORK_BODY_BYTES: usize = 256 * 1024;
const MAX_PLUGIN_NETWORK_TIMEOUT_MS: u64 = 30_000;
const PLUGIN_NETWORK_RESPONSE_TYPE_BASE64: &str = "base64";
const PLUGIN_NETWORK_RESPONSE_TYPE_TEXT: &str = "text";

pub(super) async fn execute_plugin_network_request(
    app_data_dir: PathBuf,
    plugin_id: String,
    args: PluginNetworkRequestArgs,
) -> Result<PluginNetworkResponse, String> {
    let url = plugin_network_request_url(&args.url)?;
    let method = plugin_network_request_method(args.method.as_deref())?;
    let timeout_ms = args
        .timeout_ms
        .unwrap_or(15_000)
        .clamp(1_000, MAX_PLUGIN_NETWORK_TIMEOUT_MS);
    let response_type = args
        .response_type
        .as_deref()
        .unwrap_or(PLUGIN_NETWORK_RESPONSE_TYPE_TEXT);
    if response_type != PLUGIN_NETWORK_RESPONSE_TYPE_TEXT
        && response_type != PLUGIN_NETWORK_RESPONSE_TYPE_BASE64
    {
        return Err(format!(
            "network.request responseType is unsupported: {response_type}"
        ));
    }
    let headers = plugin_network_headers(args.headers)?;
    let mut request = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|error| format!("network.request client setup failed: {error}"))?
        .request(method.clone(), url)
        .headers(headers);
    if method != reqwest::Method::GET && method != reqwest::Method::HEAD {
        match (args.body, args.body_file) {
            (Some(_), Some(_)) => {
                return Err("network.request cannot use both body and bodyFile".to_string());
            }
            (Some(body), None) => {
                if body.len() > MAX_PLUGIN_NETWORK_BODY_BYTES {
                    return Err("network.request body is too large".to_string());
                }
                request = request.body(body);
            }
            (None, Some(body_file)) => {
                let path =
                    plugin_network_body_file_path(&app_data_dir, &plugin_id, &body_file.path)?;
                let content_type =
                    plugin_network_body_file_content_type(body_file.content_type.as_deref())?;
                if let Some(content_type) = content_type {
                    request = request.header(reqwest::header::CONTENT_TYPE, content_type);
                }
                let body = std::fs::read(path)
                    .map_err(|error| format!("network.request bodyFile read failed: {error}"))?;
                request = request.body(body);
            }
            (None, None) => {}
        }
    } else if args.body.is_some() || args.body_file.is_some() {
        return Err("network.request body requires a non-GET method".to_string());
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("network.request failed: {error}"))?;
    let url = response.url().to_string();
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(key, value)| {
            value
                .to_str()
                .ok()
                .map(|header_value| (key.as_str().to_string(), header_value.to_string()))
        })
        .collect::<HashMap<_, _>>();
    if let Some(length) = response.content_length()
        && length as usize > MAX_PLUGIN_NETWORK_RESPONSE_BYTES
    {
        return Err("network.request response is too large".to_string());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("network.request response read failed: {error}"))?;
    if bytes.len() > MAX_PLUGIN_NETWORK_RESPONSE_BYTES {
        return Err("network.request response is too large".to_string());
    }
    let body_base64 = if response_type == PLUGIN_NETWORK_RESPONSE_TYPE_BASE64 {
        Some(BASE64_STANDARD.encode(&bytes))
    } else {
        None
    };
    let text = if body_base64.is_some() {
        String::new()
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    Ok(PluginNetworkResponse {
        url,
        status: status.as_u16(),
        ok: status.is_success(),
        headers,
        text,
        body_base64,
    })
}

use super::command::mpv_command_async;
use super::*;

pub(in crate::mpv_embed) fn load_media_file(
    mpv: &libmpv2::Mpv,
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<(), String> {
    let args = loadfile_args_for_media_path(path_text, load_options)?;
    let arg_refs = loadfile_arg_refs(&args);
    match mpv.command("loadfile", &arg_refs) {
        Ok(()) => Ok(()),
        Err(error) if is_hls_manifest_media_url(path_text) => {
            let legacy_args = legacy_hls_loadfile_args_for_media_path(path_text, load_options)?;
            let legacy_arg_refs = loadfile_arg_refs(&legacy_args);
            mpv.command("loadfile", &legacy_arg_refs)
                .map_err(|legacy_error| {
                    format!(
                        "mpv loadfile failed: {error}; legacy HLS loadfile failed: {legacy_error}"
                    )
                })
        }
        Err(error) => Err(format!("mpv loadfile failed: {error}")),
    }
}

pub(in crate::mpv_embed) fn load_media_file_for_interactive_open(
    mpv: &libmpv2::Mpv,
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<(), String> {
    if is_network_stream_media_url(path_text) {
        return load_media_file_async(
            mpv,
            path_text,
            load_options,
            MAIN_PLAYER_ASYNC_LOAD_REQUEST_ID,
        );
    }

    load_media_file(mpv, path_text, load_options)
}

pub(in crate::mpv_embed) fn load_media_file_async(
    mpv: &libmpv2::Mpv,
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
    request_id: u64,
) -> Result<(), String> {
    let args = loadfile_args_for_media_path(path_text, load_options)?;
    let arg_refs = loadfile_arg_refs(&args);
    mpv_command_async(mpv, request_id, "loadfile", &arg_refs)
}

fn loadfile_arg_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

pub(in crate::mpv_embed) fn loadfile_args_for_media_path(
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<Vec<String>, String> {
    let options = loadfile_options_for_media_path(path_text, load_options)?;
    let mut args = vec![path_text.to_string(), "replace".to_string()];
    if let Some(options) = options {
        args.push("-1".to_string());
        args.push(options);
    }
    Ok(args)
}

pub(in crate::mpv_embed) fn legacy_hls_loadfile_args_for_media_path(
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<Vec<String>, String> {
    let options = loadfile_options_for_media_path(path_text, load_options)?;
    let mut args = vec![path_text.to_string(), "replace".to_string()];
    if let Some(options) = options {
        args.push(options);
    }
    Ok(args)
}

fn loadfile_options_for_media_path(
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<Option<String>, String> {
    let mut options = BTreeMap::new();
    if is_hls_manifest_media_url(path_text) {
        options.insert("demuxer".to_string(), "+lavf".to_string());
        options.insert("demuxer-lavf-format".to_string(), "hls".to_string());
    }

    if let Some(load_options) = load_options {
        for (key, value) in normalize_mpv_load_options(load_options)? {
            options.insert(key, value);
        }
    }

    if options.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            options
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(","),
        ))
    }
}

fn normalize_mpv_load_options(
    load_options: &MpvLoadOptions,
) -> Result<Vec<(String, String)>, String> {
    let mut normalized = Vec::new();
    for (key, value) in &load_options.options {
        let key = key.trim().to_ascii_lowercase();
        if !is_supported_mpv_load_option_key(&key) {
            return Err(format!("unsupported mpv load option: {key}"));
        }
        if !is_valid_mpv_load_option_value(value) {
            return Err(format!("invalid mpv load option value for {key}"));
        }
        normalized.push((key, value.trim().to_string()));
    }
    Ok(normalized)
}

fn is_supported_mpv_load_option_key(key: &str) -> bool {
    matches!(key, "demuxer" | "demuxer-lavf-format")
}

fn is_valid_mpv_load_option_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && !value.contains(',')
        && !value.contains('=')
        && !value.chars().any(char::is_control)
}

pub(in crate::mpv_embed) fn is_hls_manifest_media_url(path_text: &str) -> bool {
    let Some((scheme, rest)) = path_text.trim().split_once("://") else {
        return false;
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return false;
    }
    let path_without_fragment = rest.split_once('#').map(|(path, _)| path).unwrap_or(rest);
    let path_without_query = path_without_fragment
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(path_without_fragment);
    path_without_query.to_ascii_lowercase().ends_with(".m3u8")
}

pub(in crate::mpv_embed) fn is_network_stream_media_url(path_text: &str) -> bool {
    let Some((scheme, rest)) = path_text.trim().split_once("://") else {
        return false;
    };
    !rest.trim().is_empty()
        && matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
        )
}

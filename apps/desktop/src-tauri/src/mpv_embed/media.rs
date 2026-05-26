use super::*;

pub(super) fn validate_media_path(path: &str) -> Result<PathBuf, String> {
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

pub(super) fn is_supported_media_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

pub(super) fn load_media_file(
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

pub(super) fn load_media_file_async(
    mpv: &libmpv2::Mpv,
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
    request_id: u64,
) -> Result<(), String> {
    let args = loadfile_args_for_media_path(path_text, load_options)?;
    let arg_refs = loadfile_arg_refs(&args);
    mpv_command_async(mpv, request_id, "loadfile", &arg_refs)
}

pub(super) fn mpv_command_async(
    mpv: &libmpv2::Mpv,
    request_id: u64,
    name: &str,
    args: &[&str],
) -> Result<(), String> {
    let mut cstr_args = Vec::with_capacity(args.len() + 1);
    cstr_args
        .push(CString::new(name).map_err(|error| format!("mpv command name failed: {error}"))?);

    for arg in args {
        cstr_args.push(
            CString::new(*arg).map_err(|error| format!("mpv command argument failed: {error}"))?,
        );
    }

    let mut ptrs: Vec<_> = cstr_args.iter().map(|cstr| cstr.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    let result =
        unsafe { libmpv2_sys::mpv_command_async(mpv.ctx.as_ptr(), request_id, ptrs.as_mut_ptr()) };
    if result < 0 {
        Err(format!(
            "mpv {name} async failed: {}",
            mpv_error_message(result)
        ))
    } else {
        Ok(())
    }
}

pub(super) fn loadfile_arg_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

pub(super) fn loadfile_args_for_media_path(
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

pub(super) fn legacy_hls_loadfile_args_for_media_path(
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

pub(super) fn loadfile_options_for_media_path(
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

pub(super) fn normalize_mpv_load_options(
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

pub(super) fn is_supported_mpv_load_option_key(key: &str) -> bool {
    matches!(key, "demuxer" | "demuxer-lavf-format")
}

pub(super) fn is_valid_mpv_load_option_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && !value.contains(',')
        && !value.contains('=')
        && !value.chars().any(char::is_control)
}

pub(super) fn is_hls_manifest_media_url(path_text: &str) -> bool {
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

pub(super) fn validate_subtitle_path(path: &str) -> Result<PathBuf, String> {
    let path = validate_media_path(path)?;
    if is_supported_subtitle_path(&path) {
        Ok(path)
    } else {
        Err(format!("unsupported subtitle file: {}", path.display()))
    }
}

pub(super) fn is_supported_subtitle_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_SUBTITLE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

pub(super) fn configure_audio_visualizer(mpv: &libmpv2::Mpv, path: &Path) {
    if !is_likely_audio_path(path) {
        return;
    }

    if let Err(error) = mpv.set_property("audio-display", "no") {
        eprintln!("OpenPlayer mpv audio visualizer: failed to disable cover art: {error}");
    }
}

pub(super) fn is_likely_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            AUDIO_VISUALIZER_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .unwrap_or(false)
}

pub(super) fn discover_sidecar_subtitles(media_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = media_path.parent() else {
        return Vec::new();
    };
    let Some(media_stem) = media_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    let mut subtitles: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| is_supported_subtitle_path(path))
        .filter(|path| is_matching_sidecar_stem(path, media_stem))
        .collect();

    subtitles.sort_by(|left, right| {
        sidecar_sort_key(left, media_stem).cmp(&sidecar_sort_key(right, media_stem))
    });
    subtitles
}

pub(super) fn is_matching_sidecar_stem(path: &Path, media_stem: &str) -> bool {
    let Some(candidate_stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    if candidate_stem == media_stem {
        return true;
    }

    candidate_stem
        .strip_prefix(media_stem)
        .and_then(|suffix| suffix.chars().next())
        .is_some_and(|separator| matches!(separator, '.' | '-' | '_'))
}

pub(super) fn sidecar_sort_key(path: &Path, media_stem: &str) -> (u8, String) {
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let exact_rank = if file_stem == media_stem { 0 } else { 1 };
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    (exact_rank, file_name)
}

pub(super) fn load_sidecar_subtitles(mpv: &libmpv2::Mpv, media_path: &Path) {
    for (index, subtitle) in discover_sidecar_subtitles(media_path).iter().enumerate() {
        let subtitle_text = subtitle.to_string_lossy();
        let mode = if index == 0 { "select" } else { "auto" };
        let _ = mpv.command("sub-add", &[subtitle_text.as_ref(), mode]);
    }
}

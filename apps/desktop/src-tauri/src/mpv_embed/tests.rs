use super::*;

#[test]
fn rejects_empty_media_path() {
    let error = validate_media_path("   ").expect_err("empty paths should be rejected");

    assert_eq!(error, "enter a local media path for mpv embed playback");
}

#[test]
fn accepts_supported_stream_urls_as_media_locations() {
    assert_eq!(
        validate_media_path("https://example.com/live.m3u8")
            .expect("https streams should be accepted")
            .to_string_lossy(),
        "https://example.com/live.m3u8"
    );
    assert_eq!(
        validate_media_path("rtsp://camera.local/stream")
            .expect("rtsp streams should be accepted")
            .to_string_lossy(),
        "rtsp://camera.local/stream"
    );
}

#[test]
fn rejects_unsupported_stream_urls_as_media_locations() {
    let error = validate_media_path("file://C:/secret.mp4")
        .expect_err("unsafe stream protocols should be rejected");

    assert!(error.contains("unsupported media stream protocol"));
}

#[test]
fn hls_manifest_urls_force_lavf_hls_demuxer() {
    assert!(is_hls_manifest_media_url(
        "https://ali-m-l.cztv.com/channels/lantian/channel010/1080p.m3u8"
    ));
    assert!(is_hls_manifest_media_url(
        "HTTPS://example.com/live/CHANNEL.M3U8?token=abc#frag"
    ));
    assert!(!is_hls_manifest_media_url("https://example.com/movie.mp4"));
    assert!(!is_hls_manifest_media_url("rtsp://example.com/live.m3u8"));

    assert_eq!(
        loadfile_args_for_media_path("https://example.com/live.m3u8", None)
            .expect("hls load options should be accepted"),
        vec![
            "https://example.com/live.m3u8".to_string(),
            "replace".to_string(),
            "-1".to_string(),
            "demuxer=+lavf,demuxer-lavf-format=hls".to_string()
        ]
    );
    assert_eq!(
        legacy_hls_loadfile_args_for_media_path("https://example.com/live.m3u8", None)
            .expect("legacy hls load options should be accepted"),
        vec![
            "https://example.com/live.m3u8".to_string(),
            "replace".to_string(),
            "demuxer=+lavf,demuxer-lavf-format=hls".to_string()
        ]
    );
    assert_eq!(
        loadfile_args_for_media_path("https://example.com/movie.mp4", None)
            .expect("plain media should be accepted"),
        vec![
            "https://example.com/movie.mp4".to_string(),
            "replace".to_string()
        ]
    );
}

#[test]
fn plugin_load_options_extend_safe_mpv_loadfile_options() {
    let options: MpvLoadOptions = serde_json::from_value(serde_json::json!({
        "demuxer": "+lavf",
        "demuxer-lavf-format": "hls"
    }))
    .expect("load options should deserialize from plugin hook result");

    assert_eq!(
        loadfile_args_for_media_path("https://example.com/live.custom", Some(&options))
            .expect("safe plugin load options should be accepted"),
        vec![
            "https://example.com/live.custom".to_string(),
            "replace".to_string(),
            "-1".to_string(),
            "demuxer=+lavf,demuxer-lavf-format=hls".to_string()
        ]
    );
}

#[test]
fn plugin_load_options_reject_unsafe_mpv_loadfile_options() {
    let unknown_key: MpvLoadOptions = serde_json::from_value(serde_json::json!({
        "script": "evil.lua"
    }))
    .expect("unknown key should deserialize before validation");
    assert!(
        loadfile_args_for_media_path("https://example.com/live.custom", Some(&unknown_key))
            .expect_err("unknown load option keys should be rejected")
            .contains("unsupported mpv load option")
    );

    let comma_value: MpvLoadOptions = serde_json::from_value(serde_json::json!({
        "demuxer": "+lavf,hls"
    }))
    .expect("comma value should deserialize before validation");
    assert!(
        loadfile_args_for_media_path("https://example.com/live.custom", Some(&comma_value))
            .expect_err("comma-separated option injection should be rejected")
            .contains("invalid mpv load option")
    );
}

#[test]
fn builds_sanitized_capture_output_paths() {
    let directory = PathBuf::from("captures");
    let path = capture_output_path(
        &directory,
        "https://example.com/live stream.m3u8",
        42,
        "png",
    );

    assert_eq!(
        path,
        PathBuf::from("captures").join("openplayer-live_stream-42.png")
    );
}

#[test]
fn normalizes_capture_screenshot_formats() {
    assert_eq!(normalize_capture_image_format(None).unwrap(), "png");
    assert_eq!(
        normalize_capture_image_format(Some("JPEG".to_string())).unwrap(),
        "jpg"
    );
    assert_eq!(
        normalize_capture_image_format(Some("webp".to_string())).unwrap(),
        "webp"
    );
    assert_eq!(
        normalize_capture_image_format(Some("bmp".to_string()))
            .expect_err("unsupported screenshot formats should be rejected"),
        "unsupported screenshot format: bmp"
    );
}

#[test]
fn builds_sanitized_recording_output_paths() {
    let directory = PathBuf::from("recordings");
    let path = recording_output_path(&directory, "rtsp://camera.local/live stream", 42, "mp4");

    assert_eq!(
        path,
        PathBuf::from("recordings").join("openplayer-live_stream-42.mp4")
    );
}

#[test]
fn normalizes_recording_container_formats() {
    assert_eq!(normalize_recording_container_format(None).unwrap(), "mp4");
    assert_eq!(
        normalize_recording_container_format(Some("MKV".to_string())).unwrap(),
        "mkv"
    );
    assert_eq!(
        normalize_recording_container_format(Some("ts".to_string())).unwrap(),
        "ts"
    );
    assert_eq!(
        normalize_recording_container_format(Some("avi".to_string()))
            .expect_err("unsupported recording formats should be rejected"),
        "unsupported recording format: avi"
    );
}

#[test]
fn dump_cache_recordings_preserve_requested_container() {
    assert_eq!(
        recording_container_format_for_method(
            &MpvRecordingMethod::DumpCache {
                start_position: 12.0
            },
            "mp4"
        ),
        "mp4"
    );
    assert_eq!(
        recording_container_format_for_method(
            &MpvRecordingMethod::DumpCache {
                start_position: 12.0
            },
            "ts"
        ),
        "ts"
    );
}

#[test]
fn stream_recordings_preserve_requested_container() {
    assert_eq!(
        recording_container_format_for_method(&MpvRecordingMethod::StreamRecord, "ts"),
        "ts"
    );
    assert_eq!(
        recording_container_format_for_method(&MpvRecordingMethod::StreamRecord, "mp4"),
        "mp4"
    );
}

#[test]
fn local_recordings_use_cache_dump_with_short_preroll() {
    assert_eq!(
        recording_method_for_media_path("F:\\Movies\\clip.mp4", 12.5),
        MpvRecordingMethod::DumpCache {
            start_position: 7.5
        }
    );
    assert_eq!(
        recording_method_for_media_path("F:\\Movies\\clip.mp4", 2.5),
        MpvRecordingMethod::DumpCache {
            start_position: 0.0
        }
    );
}

#[test]
fn http_network_recordings_use_cache_dump_with_short_preroll() {
    assert_eq!(
        recording_method_for_media_path("https://example.com/live.m3u8", 12.5),
        MpvRecordingMethod::DumpCache {
            start_position: 7.5
        }
    );
}

#[test]
fn live_network_recordings_use_stream_record() {
    assert_eq!(
        recording_method_for_media_path("rtsp://camera.local/live", 12.5),
        MpvRecordingMethod::StreamRecord
    );
    assert_eq!(
        recording_method_for_media_path("rtmp://example.com/live", 12.5),
        MpvRecordingMethod::StreamRecord
    );
}

#[test]
fn recording_dump_start_positions_include_bounded_preroll() {
    assert_eq!(recording_dump_start_position(8.0), 3.0);
    assert_eq!(recording_dump_start_position(3.0), 0.0);
    assert_eq!(recording_dump_start_position(f64::NAN), 0.0);
}

#[test]
fn recording_start_time_args_are_finite_and_non_negative() {
    assert_eq!(
        recording_time_arg(1.25).expect("valid recording start"),
        "1.250"
    );
    assert_eq!(
        recording_time_arg(-4.5).expect("negative starts should clamp"),
        "0.000"
    );
    assert_eq!(
        recording_time_arg(f64::NAN).expect_err("invalid starts should fail"),
        "recording start time is invalid"
    );
}

#[test]
fn rejects_empty_recording_outputs() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-empty-recording-{}-{}",
        std::process::id(),
        current_time_ms_for_capture()
    ));
    std::fs::create_dir_all(&directory).expect("temp recording directory should be created");
    let output_path = directory.join("empty.mp4");
    std::fs::write(&output_path, []).expect("empty recording file should be written");

    let error = ensure_recording_output_has_content(&output_path)
        .expect_err("empty recording outputs should fail");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("empty recording file"));
}

#[test]
fn polling_empty_recording_outputs_does_not_delete_them() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-empty-recording-poll-{}-{}",
        std::process::id(),
        current_time_ms_for_capture()
    ));
    std::fs::create_dir_all(&directory).expect("temp recording directory should be created");
    let output_path = directory.join("empty.mp4");
    std::fs::write(&output_path, []).expect("empty recording file should be written");

    assert!(!recording_output_has_content(&output_path).expect("empty file should be readable"));
    assert!(output_path.exists());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn accepts_custom_capture_directory_overrides() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-capture-directory-{}-{}",
        std::process::id(),
        current_time_ms_for_capture()
    ));
    let resolved =
        normalize_capture_directory_override(Some(directory.to_string_lossy().to_string()))
            .expect("custom capture directory should normalize");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(resolved, Some(directory));
}

#[test]
fn rejects_file_capture_directory_overrides() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-capture-directory-file-{}-{}",
        std::process::id(),
        current_time_ms_for_capture()
    ));
    std::fs::create_dir_all(&directory).expect("temp capture directory should be created");
    let file_path = directory.join("not-a-directory.txt");
    std::fs::write(&file_path, b"fixture").expect("temp file should be written");

    let error = normalize_capture_directory_override(Some(file_path.to_string_lossy().to_string()))
        .expect_err("file capture directory overrides should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("capture directory path is not a directory"));
}

#[test]
#[cfg(windows)]
fn encodes_win32_class_name_with_null_terminator() {
    let encoded = wide_null("STATIC");

    assert_eq!(encoded.last(), Some(&0));
    assert_eq!(encoded[..6], [83, 84, 65, 84, 73, 67]);
}

#[test]
fn clamps_supported_playback_speed_range() {
    assert_eq!(normalize_playback_speed(0.1).unwrap(), MIN_PLAYBACK_SPEED);
    assert_eq!(normalize_playback_speed(1.25).unwrap(), 1.25);
    assert_eq!(normalize_playback_speed(8.0).unwrap(), MAX_PLAYBACK_SPEED);
    assert_eq!(
        normalize_playback_speed(f64::NAN).expect_err("nan should be rejected"),
        "invalid mpv playback speed"
    );
}

#[test]
fn clamps_supported_subtitle_delay_range() {
    assert_eq!(normalize_subtitle_delay(-30.0).unwrap(), MIN_SUBTITLE_DELAY);
    assert_eq!(normalize_subtitle_delay(0.15).unwrap(), 0.15);
    assert_eq!(normalize_subtitle_delay(45.0).unwrap(), MAX_SUBTITLE_DELAY);
    assert_eq!(
        normalize_subtitle_delay(f64::NAN).expect_err("nan should be rejected"),
        "invalid mpv subtitle delay"
    );
}

#[test]
fn maps_hardware_decoding_modes_to_mpv_hwdec_values() {
    assert_eq!(normalize_hwdec_mode("hardware").unwrap(), "auto-safe");
    assert_eq!(normalize_hwdec_mode("software").unwrap(), "no");
    assert_eq!(normalize_hwdec_mode("auto-safe").unwrap(), "auto-safe");
    assert_eq!(normalize_hwdec_mode("no").unwrap(), "no");
    assert_eq!(
        normalize_hwdec_mode("gpu-next").expect_err("unsupported modes should be rejected"),
        "invalid mpv hardware decoding mode"
    );
}

#[test]
fn normalizes_initial_resume_positions() {
    assert_eq!(normalize_initial_resume_position(Some(42.0)), Some(42.0));
    assert_eq!(normalize_initial_resume_position(Some(0.0)), None);
    assert_eq!(normalize_initial_resume_position(Some(-1.0)), None);
    assert_eq!(normalize_initial_resume_position(Some(f64::NAN)), None);
    assert_eq!(normalize_initial_resume_position(None), None);
}

#[test]
fn normalizes_initial_volume_before_media_load() {
    assert_eq!(normalize_initial_volume(None).unwrap(), DEFAULT_VOLUME);
    assert_eq!(normalize_initial_volume(Some(0.0)).unwrap(), 0.0);
    assert_eq!(normalize_initial_volume(Some(150.0)).unwrap(), 100.0);
    assert_eq!(
        normalize_initial_volume(Some(f64::NAN)).expect_err("nan volume should be rejected"),
        "invalid mpv volume"
    );
}

#[test]
fn waits_for_initial_resume_seek_until_duration_and_seekability_are_ready() {
    assert_eq!(
        initial_resume_seek_readiness(120.0, 0.0, true),
        InitialResumeSeekReadiness::Ready
    );
    assert_eq!(
        initial_resume_seek_readiness(120.0, 600.0, false),
        InitialResumeSeekReadiness::Ready
    );
    assert_eq!(
        initial_resume_seek_readiness(120.0, 600.0, true),
        InitialResumeSeekReadiness::Ready
    );
}

#[test]
fn waits_instead_of_skipping_when_early_duration_is_shorter_than_resume_target() {
    assert_eq!(
        initial_resume_seek_readiness(1800.0, 30.0, false),
        InitialResumeSeekReadiness::Wait
    );
    assert_eq!(
        initial_resume_seek_readiness(1800.0, 30.0, true),
        InitialResumeSeekReadiness::Ready
    );
}

#[test]
fn treats_early_mpv_command_rejection_as_transient_resume_seek_failure() {
    assert!(is_transient_initial_resume_seek_error(
        &libmpv2::Error::Raw(libmpv2::mpv_error::Command)
    ));
    assert!(!is_transient_initial_resume_seek_error(
        &libmpv2::Error::Raw(libmpv2::mpv_error::Generic)
    ));
}

#[test]
fn skips_initial_resume_seek_only_when_target_is_invalid() {
    assert_eq!(
        initial_resume_seek_readiness(0.0, 600.0, true),
        InitialResumeSeekReadiness::Skip
    );
    assert_eq!(
        initial_resume_seek_readiness(f64::NAN, 600.0, true),
        InitialResumeSeekReadiness::Skip
    );
}

#[test]
fn discovers_same_stem_sidecar_subtitles() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-sidecars-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("temp subtitle directory should be created");

    let media = directory.join("episode.mkv");
    std::fs::write(&media, b"media").expect("media fixture should be written");
    std::fs::write(directory.join("episode.srt"), b"subtitle")
        .expect("subtitle fixture should be written");
    std::fs::write(directory.join("episode.zh-CN.ass"), b"subtitle")
        .expect("language subtitle fixture should be written");
    std::fs::write(directory.join("episode.notes.txt"), b"notes")
        .expect("non-subtitle fixture should be written");
    std::fs::write(directory.join("other.srt"), b"subtitle")
        .expect("unrelated subtitle fixture should be written");

    let names: Vec<String> = discover_sidecar_subtitles(&media)
        .into_iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("subtitle file name should be utf-8")
                .to_string()
        })
        .collect();

    let _ = std::fs::remove_dir_all(&directory);
    assert_eq!(names, vec!["episode.srt", "episode.zh-CN.ass"]);
}

#[test]
fn enables_real_audio_visualizer_for_audio_files_only() {
    assert!(is_likely_audio_path(Path::new("song.MP3")));
    assert!(is_likely_audio_path(Path::new("voice.amr")));
    assert!(is_likely_audio_path(Path::new("audiobook.m4b")));
    assert!(is_likely_audio_path(Path::new("sample.caf")));
    assert!(is_likely_audio_path(Path::new("album.track.flac")));
    assert!(is_likely_audio_path(Path::new("mix.opus")));
    assert!(!is_likely_audio_path(Path::new("movie.mp4")));
    assert!(!is_likely_audio_path(Path::new("clip.mkv")));
}

#[test]
fn maps_track_kinds_to_mpv_properties() {
    assert_eq!(track_property_for_kind("audio").unwrap(), "aid");
    assert_eq!(track_property_for_kind("video").unwrap(), "vid");
    assert_eq!(track_property_for_kind("subtitle").unwrap(), "sid");
    assert_eq!(track_property_for_kind("sub").unwrap(), "sid");
    assert_eq!(
        track_property_for_kind("chapter").expect_err("unsupported kinds should be rejected"),
        "invalid mpv track kind"
    );
}

#[test]
fn normalizes_plugin_owned_mpv_properties() {
    assert_eq!(
        normalize_plugin_mpv_property("sub-font-size", &serde_json::json!(52)).unwrap(),
        ("sub-font-size", PluginMpvPropertyValue::Number(52.0))
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-font", &serde_json::json!("Inter")).unwrap(),
        (
            "sub-font",
            PluginMpvPropertyValue::Text("Inter".to_string())
        )
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-color", &serde_json::json!("#78d5b3")).unwrap(),
        (
            "sub-color",
            PluginMpvPropertyValue::Text("#78d5b3".to_string())
        )
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(4)).unwrap(),
        ("sub-spacing", PluginMpvPropertyValue::Text("4".to_string()))
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(10)).unwrap(),
        (
            "sub-spacing",
            PluginMpvPropertyValue::Text("10".to_string())
        )
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-border-size", &serde_json::json!(2.5)).unwrap(),
        ("sub-outline-size", PluginMpvPropertyValue::Number(2.5))
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-shadow-offset", &serde_json::json!(1.5)).unwrap(),
        ("sub-shadow-offset", PluginMpvPropertyValue::Number(1.5))
    );
}

#[test]
fn rejects_plugin_owned_mpv_properties_outside_allowlist() {
    assert_eq!(
        normalize_plugin_mpv_property("vf", &serde_json::json!("lavfi=[scale=2]"))
            .expect_err("plugins must not set arbitrary mpv properties"),
        "unsupported plugin mpv property: vf"
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-font-size", &serde_json::json!(999))
            .expect_err("subtitle font size outside the allowed range should be rejected"),
        "invalid plugin subtitle font size"
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(11))
            .expect_err("subtitle spacing above mpv's stable range should be rejected"),
        "invalid plugin subtitle spacing"
    );
}

#[test]
fn plugin_subtitle_style_properties_force_ass_overrides() {
    assert!(plugin_subtitle_style_requires_ass_override("sub-font-size"));
    assert!(plugin_subtitle_style_requires_ass_override("sub-spacing"));
    assert!(!plugin_subtitle_style_requires_ass_override(
        "sub-line-spacing"
    ));
    assert!(!plugin_subtitle_style_requires_ass_override("sub-delay"));
}

#[test]
fn subtitle_spacing_writes_only_stable_mpv_property() {
    assert_eq!(
        plugin_mpv_property_write_targets("sub-line-spacing"),
        &[] as &[&str]
    );
    assert_eq!(
        plugin_mpv_property_write_targets("sub-spacing"),
        &["sub-spacing"]
    );
}

#[test]
fn prepares_numeric_locale_for_libmpv_initialization() {
    assert!(prepare_libmpv_numeric_locale().is_ok());
}

#[test]
fn linux_video_output_falls_back_to_x11_when_dri_render_node_is_missing() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: None,
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: false,
        virtual_drm_driver: false,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("x11".to_string()),
            gpu_context: None,
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn linux_video_output_falls_back_to_x11_for_virtual_drm_drivers() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: None,
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: true,
        virtual_drm_driver: true,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("x11".to_string()),
            gpu_context: None,
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn linux_video_output_uses_x11egl_when_dri_render_node_is_available() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: None,
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: true,
        virtual_drm_driver: false,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("gpu".to_string()),
            gpu_context: Some("x11egl".to_string()),
            hwdec: "auto-safe".to_string(),
        }
    );
}

#[test]
fn linux_video_output_allows_field_vo_override() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: Some("x11"),
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: true,
        virtual_drm_driver: false,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("x11".to_string()),
            gpu_context: None,
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn linux_video_output_allows_gpu_context_and_hwdec_overrides() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: Some("gpu"),
        override_gpu_context: Some("x11"),
        override_hwdec: Some("no"),
        has_dri_render_node: false,
        virtual_drm_driver: true,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("gpu".to_string()),
            gpu_context: Some("x11".to_string()),
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn identifies_known_virtual_linux_drm_drivers() {
    assert!(is_virtual_linux_drm_driver("bochs-drm"));
    assert!(is_virtual_linux_drm_driver("QXL"));
    assert!(is_virtual_linux_drm_driver("virtio_gpu"));
    assert!(!is_virtual_linux_drm_driver("i915"));
    assert!(!is_virtual_linux_drm_driver("amdgpu"));
}

#[test]
fn forwards_mpv_video_diagnostic_log_messages() {
    assert!(is_mpv_video_diagnostic_log(
        "warn",
        "vo/gpu",
        "libEGL warning: DRI3 error: Could not get DRI3 device"
    ));
    assert!(is_mpv_video_diagnostic_log(
        "info",
        "cplayer",
        "VO: [x11] 1280x720 yuv420p"
    ));
    assert!(is_mpv_video_diagnostic_log(
        "v",
        "vd",
        "Trying hardware decoding via vaapi"
    ));
    assert!(!is_mpv_video_diagnostic_log(
        "info",
        "cplayer",
        "Playing: sample.mp4"
    ));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_video_output_uses_libmpv_render_api_vo() {
    let config = platform_video_output_config();

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("libmpv".to_string()),
            gpu_context: None,
            hwdec: "auto-safe".to_string(),
        }
    );
}

#[test]
fn maps_x11_window_handles_to_mpv_wid_values() {
    let xlib = RawWindowHandle::Xlib(raw_window_handle::XlibWindowHandle::new(42));
    assert_eq!(mpv_wid_from_raw_window_handle(xlib).unwrap(), 42);

    let xcb_window = std::num::NonZeroU32::new(84).expect("fixture window id is non-zero");
    let xcb = RawWindowHandle::Xcb(raw_window_handle::XcbWindowHandle::new(xcb_window));
    assert_eq!(mpv_wid_from_raw_window_handle(xcb).unwrap(), 84);
}

#[test]
fn rejects_wayland_until_native_host_exists() {
    let surface = std::ptr::NonNull::dangling();
    let handle = RawWindowHandle::Wayland(raw_window_handle::WaylandWindowHandle::new(surface));

    assert_eq!(
        mpv_wid_from_raw_window_handle(handle).expect_err("Wayland does not support mpv wid"),
        "mpv embed playback currently supports Windows HWND and X11 window hosts; Wayland video host support is not implemented yet"
    );
}

#[test]
fn wall_tile_requests_accept_rtsp_rtmp_http_and_hls_streams() {
    for url in [
        "rtsp://example.test/live/one",
        "rtmp://example.test/live/two",
        "http://example.test/live/three.mp4",
        "https://example.test/live/four.m3u8",
        "rtsp://[240e:39f:3a6:6f70:cc97:60e7:d590:7281]:8554/webm_rtsp_1",
        "rtmp://[240e:39f:3a6:6f70:cc97:60e7:d590:7281]:19350/webm_rtmp",
    ] {
        let tile = MpvWallTileRequest {
            id: "tile-1".to_string(),
            url: url.to_string(),
            title: Some("Camera".to_string()),
            x: 0.0,
            y: 0.0,
            width: 0.5,
            height: 0.5,
            muted: Some(true),
        };

        assert_eq!(normalize_wall_tile_request(tile).unwrap().url, url);
    }
}

#[test]
fn wall_tile_fraction_layout_maps_to_parent_pixels() {
    let rect = normalize_wall_tile_rect(0.25, 0.5, 0.5, 0.25).unwrap();
    let layout = wall_tile_rect_to_video_host_rect(1920, 1080, rect);

    assert_eq!(
        layout,
        VideoHostRect {
            x: 480,
            y: 540,
            width: 960,
            height: 270,
        }
    );
}

#[test]
fn wall_open_initial_snapshots_cover_every_tile_before_players_start() {
    let tiles = normalize_wall_tile_requests(vec![
        MpvWallTileRequest {
            id: "rtsp-one".to_string(),
            url: "rtsp://example.test/live/one".to_string(),
            title: Some("One".to_string()),
            x: 0.0,
            y: 0.0,
            width: 0.5,
            height: 0.5,
            muted: Some(true),
        },
        MpvWallTileRequest {
            id: "rtmp-two".to_string(),
            url: "rtmp://example.test/live/two".to_string(),
            title: Some("Two".to_string()),
            x: 0.5,
            y: 0.0,
            width: 0.5,
            height: 0.5,
            muted: Some(true),
        },
    ])
    .unwrap();

    let snapshots = wall_initial_snapshots(&tiles);

    assert_eq!(snapshots.len(), 2);
    assert!(
        snapshots
            .iter()
            .all(|snapshot| snapshot.status == "loading")
    );
    assert_eq!(snapshots[0].id, "rtsp-one");
    assert_eq!(snapshots[1].id, "rtmp-two");
}

#[test]
fn wall_live_status_keeps_terminal_and_loading_states_stable() {
    assert_eq!(wall_live_status(true, false, false), "ended");
    assert_eq!(wall_live_status(false, true, false), "paused");
    assert_eq!(wall_live_status(false, false, true), "loading");
    assert_eq!(wall_live_status(false, false, false), "playing");
}

#[test]
fn wall_bitrate_prefers_track_bitrates_and_falls_back_to_raw_input_rate() {
    assert_eq!(
        combine_wall_bitrate(Some(4_000_000.0), Some(160_000.0), Some(100_000.0)),
        Some(4_160_000.0)
    );
    assert_eq!(
        combine_wall_bitrate(None, None, Some(250_000.0)),
        Some(2_000_000.0)
    );
    assert_eq!(combine_wall_bitrate(Some(0.0), None, Some(-1.0)), None);
}

#[test]
fn wall_osd_formats_buffer_in_milliseconds() {
    assert_eq!(format_wall_buffer_millis(Some(0.021)), "21 ms");
    assert_eq!(format_wall_buffer_millis(Some(1.234)), "1234 ms");
    assert_eq!(format_wall_buffer_millis(None), "-- ms");
}

#[test]
fn wall_osd_formats_bitrate_compactly() {
    assert_eq!(format_wall_bitrate(Some(2_500_000.0)), "2.5 Mbps");
    assert_eq!(format_wall_bitrate(Some(640_000.0)), "640 Kbps");
    assert_eq!(format_wall_bitrate(None), "--");
}

#[test]
fn wall_request_ids_are_unique_per_generation_and_tile() {
    assert_eq!(wall_request_id(7, 0), 7_001);
    assert_eq!(wall_request_id(7, 1), 7_002);
    assert_ne!(wall_request_id(7, 0), wall_request_id(8, 0));
}

#[test]
#[cfg(windows)]
fn wall_open_reuses_same_tile_set_without_resetting_generation() {
    let state = MpvWallState::default();
    let tiles = normalize_wall_tile_requests(vec![MpvWallTileRequest {
        id: "rtsp-one".to_string(),
        url: "rtsp://example.test/live/one".to_string(),
        title: Some("One".to_string()),
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        muted: Some(true),
    }])
    .unwrap();

    let generation = state.next_generation().unwrap();
    state
        .replace_opening_state(wall_initial_snapshots(&tiles))
        .unwrap();

    assert!(state.can_reuse_open_wall(&tiles).unwrap());
    assert_eq!(state.current_generation().unwrap(), generation);
}

#[test]
#[cfg(windows)]
fn wall_starting_guard_prevents_duplicate_tile_starts() {
    let state = MpvWallState::default();
    let generation = state.next_generation().unwrap();

    assert!(state.mark_tile_starting(generation, "camera-1").unwrap());
    assert!(!state.mark_tile_starting(generation, "camera-1").unwrap());
    state.clear_tile_starting("camera-1").unwrap();
    assert!(state.mark_tile_starting(generation, "camera-1").unwrap());
}

#[test]
#[cfg(windows)]
fn wall_take_players_clears_players_without_resetting_generation() {
    let state = MpvWallState::default();
    let generation = state.next_generation().unwrap();

    let players = state.take_players().unwrap();

    assert!(players.is_empty());
    assert_eq!(state.current_generation().unwrap(), generation);
}

#[test]
#[cfg(windows)]
fn reserves_web_controls_outside_native_video_host() {
    let rect = video_host_rect(1280, 720);

    assert_eq!(rect.x, 0);
    assert_eq!(rect.y, 0);
    assert_eq!(rect.width, 1280);
    assert_eq!(rect.height, 720);
}

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
fn identifies_network_stream_urls_for_async_interactive_loads() {
    assert!(is_network_stream_media_url("https://example.com/live.m3u8"));
    assert!(is_network_stream_media_url("rtsp://camera.local/stream"));
    assert!(is_network_stream_media_url("srt://example.com:9000"));
    assert!(!is_network_stream_media_url("C:\\Media\\movie.mp4"));
    assert!(!is_network_stream_media_url("file://C:/secret.mp4"));
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
fn writes_generated_subtitle_files_in_plugin_scoped_directory() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-generated-subtitles-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    ));

    let content = "1\n00:00:00,000 --> 00:00:01,000\nHello from a plugin\n";
    let path = write_generated_subtitle_file(
        &directory,
        "dev.openplayer.ai-transcript",
        Some("../Live Transcript: Segment 01"),
        "SRT",
        content,
    )
    .expect("generated subtitle should be written");

    let scoped_directory = directory
        .join("generated-subtitles")
        .join("dev.openplayer.ai-transcript");
    assert!(path.starts_with(&scoped_directory));
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some("srt")
    );
    assert!(
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .ends_with("-live-transcript-segment-01.srt")
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("generated subtitle should be readable"),
        content
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn generated_subtitle_management_is_limited_to_owning_plugin_paths() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-generated-subtitle-ownership-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    ));
    let owned_path = write_generated_subtitle_file(
        &directory,
        "dev.openplayer.transcriber",
        Some("live transcript"),
        "srt",
        "1\n00:00:00,000 --> 00:00:01,000\nOwned\n",
    )
    .expect("owned generated subtitle should be written");
    let outside_path = directory.join("outside.srt");
    std::fs::write(&outside_path, "outside").expect("outside subtitle should be written");

    let owned = plugin_generated_subtitle_path(
        &directory,
        "dev.openplayer.transcriber",
        &owned_path.to_string_lossy(),
    )
    .expect("owning plugin should be allowed to manage its generated subtitle");
    let cross_plugin_error = plugin_generated_subtitle_path(
        &directory,
        "dev.openplayer.translator",
        &owned_path.to_string_lossy(),
    )
    .expect_err("another plugin must not manage this generated subtitle");
    let outside_error = plugin_generated_subtitle_path(
        &directory,
        "dev.openplayer.transcriber",
        &outside_path.to_string_lossy(),
    )
    .expect_err("plugin must not manage arbitrary subtitle files");
    let owned_canonical = owned_path
        .canonicalize()
        .expect("owned generated subtitle should canonicalize");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(owned, owned_canonical);
    assert!(cross_plugin_error.contains("not owned by the current plugin"));
    assert!(outside_error.contains("not owned by the current plugin"));
}

#[test]
fn rejects_invalid_generated_subtitle_requests() {
    let directory = std::env::temp_dir();

    assert!(
        write_generated_subtitle_file(
            &directory,
            "dev.openplayer.ai-transcript",
            Some("transcript"),
            "exe",
            "subtitle"
        )
        .expect_err("unsupported generated subtitle formats should be rejected")
        .contains("unsupported generated subtitle format")
    );

    let oversized = "x".repeat(MAX_GENERATED_SUBTITLE_BYTES + 1);
    assert!(
        write_generated_subtitle_file(
            &directory,
            "dev.openplayer.ai-transcript",
            Some("transcript"),
            "srt",
            &oversized
        )
        .expect_err("oversized generated subtitles should be rejected")
        .contains("too large")
    );
}

#[test]
fn formats_structured_generated_subtitle_cues() {
    let cues = vec![
        GeneratedSubtitleCue {
            start: 2.5,
            end: 4.25,
            text: " second line ".to_string(),
        },
        GeneratedSubtitleCue {
            start: 0.0,
            end: 1.234,
            text: " Hello\nworld ".to_string(),
        },
    ];

    let srt =
        format_generated_subtitle_cues("srt", &cues).expect("structured cues should format as srt");
    assert!(srt.contains("1\n00:00:00,000 --> 00:00:01,234\nHello\nworld"));
    assert!(srt.contains("2\n00:00:02,500 --> 00:00:04,250\nsecond line"));

    let vtt =
        format_generated_subtitle_cues("vtt", &cues).expect("structured cues should format as vtt");
    assert!(vtt.starts_with("WEBVTT\n\n"));
    assert!(vtt.contains("00:00:00.000 --> 00:00:01.234\nHello\nworld"));
}

#[test]
fn rejects_invalid_structured_generated_subtitle_cues() {
    let invalid_timing = vec![GeneratedSubtitleCue {
        start: 3.0,
        end: 3.0,
        text: "stuck".to_string(),
    }];
    assert!(
        format_generated_subtitle_cues("srt", &invalid_timing)
            .expect_err("zero-length cues should be rejected")
            .contains("start/end")
    );

    let empty_text = vec![GeneratedSubtitleCue {
        start: 0.0,
        end: 1.0,
        text: "   ".to_string(),
    }];
    assert!(
        format_generated_subtitle_cues("vtt", &empty_text)
            .expect_err("empty cue text should be rejected")
            .contains("non-empty text")
    );

    assert!(
        format_generated_subtitle_cues("ass", &empty_text)
            .expect_err("structured cues should allow only direct cue formats")
            .contains("unsupported generated subtitle cue format")
    );
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

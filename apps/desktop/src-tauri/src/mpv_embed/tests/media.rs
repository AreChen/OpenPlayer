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

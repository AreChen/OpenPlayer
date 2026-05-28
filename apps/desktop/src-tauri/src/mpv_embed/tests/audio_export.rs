use super::*;

#[test]
fn normalizes_plugin_audio_clip_requests_for_transcription_chunks() {
    let request = normalize_audio_clip_extract_request(
        Some(12.34567),
        Some(4.5),
        Some(16_000),
        Some("mono".to_string()),
        true,
    )
    .expect("short mono transcription chunks should normalize");

    assert_eq!(request.start, 12.346);
    assert_eq!(request.duration, 4.5);
    assert_eq!(request.sample_rate, 16_000);
    assert_eq!(request.channels, AudioClipChannels::Mono);
    assert!(request.include_base64);
    assert_eq!(request.format, "wav");
}

#[test]
fn rejects_unsafe_plugin_audio_clip_requests() {
    assert!(
        normalize_audio_clip_extract_request(Some(-1.0), Some(1.0), Some(16_000), None, false)
            .expect_err("negative starts should be rejected")
            .contains("audio clip start")
    );
    assert!(
        normalize_audio_clip_extract_request(None, Some(60.0), Some(16_000), None, false)
            .expect_err("long clips should be rejected")
            .contains("audio clip duration")
    );
    assert!(
        normalize_audio_clip_extract_request(None, Some(1.0), Some(44_100), None, false)
            .expect_err("unsupported sample rates should be rejected")
            .contains("audio clip sampleRate")
    );
    assert!(
        normalize_audio_clip_extract_request(
            None,
            Some(1.0),
            Some(16_000),
            Some("5.1".to_string()),
            false
        )
        .expect_err("unsupported channel layouts should be rejected")
        .contains("audio clip channels")
    );
}

#[test]
fn builds_plugin_scoped_audio_clip_output_paths() {
    let path = audio_clip_output_path(
        &PathBuf::from("app-data"),
        "dev.openplayer.ai-transcript",
        "C:\\Media Library\\Episode 01.mkv",
        42,
    )
    .expect("plugin audio clip output path should normalize");

    assert_eq!(
        path,
        PathBuf::from("app-data")
            .join("audio-clips")
            .join("dev.openplayer.ai-transcript")
            .join("openplayer-Episode_01-42.wav")
    );
}

#[test]
fn rejects_invalid_audio_clip_plugin_ids() {
    assert!(
        audio_clip_output_path(&PathBuf::from("app-data"), "../plugin", "movie.mkv", 42)
            .expect_err("path-like plugin ids should be rejected")
            .contains("invalid audio clip plugin id")
    );
}

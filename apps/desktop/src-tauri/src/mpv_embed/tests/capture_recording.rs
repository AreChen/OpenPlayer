use super::*;

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
fn builds_plugin_scoped_frame_capture_output_paths() {
    let path = plugin_frame_capture_output_path(
        &PathBuf::from("app-data"),
        "dev.openplayer.ocr",
        "C:\\Media Library\\Episode 01.mkv",
        42,
        "webp",
    )
    .expect("plugin frame capture output path should normalize");

    assert_eq!(
        path,
        PathBuf::from("app-data")
            .join("frame-captures")
            .join("dev.openplayer.ocr")
            .join("openplayer-Episode_01-42.webp")
    );
}

#[test]
fn rejects_invalid_frame_capture_plugin_ids() {
    assert!(
        plugin_frame_capture_output_path(
            &PathBuf::from("app-data"),
            "../plugin",
            "movie.mkv",
            42,
            "png"
        )
        .expect_err("path-like plugin ids should be rejected")
        .contains("invalid frame capture plugin id")
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

use super::*;
use std::process::{Command, Stdio};

#[test]
fn normalizes_media_segment_export_requests_for_audio_and_video() {
    let audio = normalize_media_segment_export_request(
        Some("audio".to_string()),
        Some("MP3".to_string()),
        Some(12.34567),
        Some(8.0),
        None,
    )
    .expect("mp3 audio segment exports should normalize");

    assert_eq!(audio.kind, MediaSegmentExportKind::Audio);
    assert_eq!(audio.start, 12.346);
    assert_eq!(audio.duration, 8.0);
    assert_eq!(audio.format.id, "mp3");
    assert_eq!(audio.format.extension, "mp3");
    assert_eq!(audio.format.mime_type, "audio/mpeg");

    let video = normalize_media_segment_export_request(
        Some("video".to_string()),
        Some("MKV".to_string()),
        Some(1.0),
        Some(30.0),
        Some("Scene 01.mkv".to_string()),
    )
    .expect("mkv video segment exports should normalize");

    assert_eq!(video.kind, MediaSegmentExportKind::Video);
    assert_eq!(video.format.id, "mkv");
    assert_eq!(video.format.extension, "mkv");
    assert_eq!(video.format.mime_type, "video/x-matroska");
    assert_eq!(video.file_stem.as_deref(), Some("Scene_01"));
}

#[test]
fn rejects_unsafe_media_segment_export_requests() {
    assert!(
        normalize_media_segment_export_request(
            Some("subtitle".to_string()),
            Some("mp3".to_string()),
            Some(0.0),
            Some(1.0),
            None,
        )
        .expect_err("unsupported export kinds should be rejected")
        .contains("media segment export kind")
    );
    assert!(
        normalize_media_segment_export_request(
            Some("audio".to_string()),
            Some("mp4".to_string()),
            Some(0.0),
            Some(1.0),
            None,
        )
        .expect_err("audio export should reject video formats")
        .contains("unsupported audio segment export format")
    );
    assert!(
        normalize_media_segment_export_request(
            Some("video".to_string()),
            Some("mp3".to_string()),
            Some(0.0),
            Some(1.0),
            None,
        )
        .expect_err("video export should reject audio formats")
        .contains("unsupported video segment export format")
    );
    assert!(
        normalize_media_segment_export_request(
            Some("audio".to_string()),
            Some("mp3".to_string()),
            Some(-1.0),
            Some(1.0),
            None,
        )
        .expect_err("negative starts should be rejected")
        .contains("start")
    );
    assert!(
        normalize_media_segment_export_request(
            Some("video".to_string()),
            Some("mp4".to_string()),
            Some(0.0),
            Some(900.0),
            None,
        )
        .expect_err("overly long exports should be rejected")
        .contains("duration")
    );
}

#[test]
fn builds_sanitized_media_segment_export_output_paths() {
    let request = normalize_media_segment_export_request(
        Some("audio".to_string()),
        Some("mp3".to_string()),
        Some(0.0),
        Some(5.0),
        Some("../Line 01?.mp3".to_string()),
    )
    .expect("custom export filenames should sanitize");
    let path = media_segment_export_output_path(
        &PathBuf::from("exports"),
        "C:\\Media Library\\Episode 01.mkv",
        42,
        &request,
    );

    assert_eq!(
        path,
        PathBuf::from("exports").join("openplayer-Line_01-42.mp3")
    );

    let default_name = normalize_media_segment_export_request(
        Some("video".to_string()),
        Some("mp4".to_string()),
        Some(0.0),
        Some(5.0),
        None,
    )
    .expect("default export filenames should normalize");
    let path = media_segment_export_output_path(
        &PathBuf::from("exports"),
        "C:\\Media Library\\Episode 01.mkv",
        42,
        &default_name,
    );

    assert_eq!(
        path,
        PathBuf::from("exports").join("openplayer-Episode_01-42.mp4")
    );
}

#[test]
fn rejects_tiny_media_segment_export_outputs() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-tiny-segment-export-{}-{}",
        std::process::id(),
        current_time_ms_for_capture()
    ));
    std::fs::create_dir_all(&directory).expect("temp export directory should be created");
    let output_path = directory.join("empty-header.mp4");
    std::fs::write(&output_path, [0_u8; 48]).expect("tiny output should be written");

    let error = ensure_media_segment_export_output(&output_path)
        .expect_err("tiny header-only exports should fail");

    assert!(error.contains("empty media segment export"));
    assert!(!output_path.exists(), "invalid output should be removed");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn video_segment_exports_preserve_video_frames_and_release_output_file() {
    if !command_available("ffmpeg") {
        return;
    }

    let directory = std::env::temp_dir().join(format!(
        "openplayer-segment-export-{}-{}",
        std::process::id(),
        current_time_ms_for_capture()
    ));
    std::fs::create_dir_all(&directory).expect("temp export directory should be created");
    let input_path = directory.join("red-input.mp4");
    let output_path = directory.join("red-output.mp4");

    let input_status = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=160x90:r=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "1",
            "-shortest",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&input_path)
        .status()
        .expect("ffmpeg should create a test input");
    assert!(input_status.success(), "ffmpeg should create a test input");

    let request = normalize_media_segment_export_request(
        Some("video".to_string()),
        Some("mp4".to_string()),
        Some(0.0),
        Some(0.75),
        None,
    )
    .expect("mp4 video segment exports should normalize");

    let artifact =
        export_media_segment_to_file(&input_path.to_string_lossy(), &output_path, &request)
            .expect("video segment should export");

    assert_eq!(artifact.kind, "video");
    assert_eq!(artifact.format, "mp4");
    assert!(
        exported_first_frame_average_red(&output_path) > 32.0,
        "exported video frame should not be black"
    );
    std::fs::remove_file(&output_path).expect("export output should not remain locked by mpv");
    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_dir_all(directory);
}

fn command_available(command: &str) -> bool {
    Command::new(command)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn exported_first_frame_average_red(path: &Path) -> f64 {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .expect("ffmpeg should decode exported first frame");
    assert!(
        output.status.success(),
        "ffmpeg should decode exported first frame"
    );
    assert!(
        !output.stdout.is_empty(),
        "decoded frame should contain RGB bytes"
    );

    let red_sum: u64 = output
        .stdout
        .chunks_exact(3)
        .map(|pixel| u64::from(pixel[0]))
        .sum();
    red_sum as f64 / (output.stdout.len() / 3) as f64
}

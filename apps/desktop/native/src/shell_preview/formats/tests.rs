use std::collections::HashSet;

use super::*;

#[test]
fn preview_format_catalog_has_no_duplicate_extensions() {
    let mut extensions = HashSet::new();
    for format in PREVIEW_FORMATS {
        assert!(
            extensions.insert(format.extension),
            "duplicate preview extension: {}",
            format.extension
        );
    }
}

#[test]
fn preview_format_catalog_covers_common_mpv_media_containers() {
    let extensions: HashSet<&str> = PREVIEW_FORMATS
        .iter()
        .map(|format| format.extension)
        .collect();

    for extension in [
        "mp4", "mkv", "avi", "webm", "mov", "flv", "m2ts", "vob", "mxf", "mp3", "flac", "m4a",
        "m4b", "amr", "caf", "spx", "opus", "wav", "wma", "wv",
    ] {
        assert!(
            extensions.contains(extension),
            "missing preview extension: {extension}"
        );
    }
}

#[test]
fn registration_summary_counts_video_and_audio_formats() {
    let summary = registration_summary(PREVIEW_FORMATS);

    assert_eq!(summary.registered_count, PREVIEW_FORMATS.len());
    assert!(summary.video_count > 40);
    assert!(summary.audio_count > 30);
    assert_eq!(
        summary.registered_count,
        summary.video_count + summary.audio_count
    );
}

#[test]
fn preview_format_catalog_marks_common_defaults_as_subset() {
    let formats: Vec<ShellPreviewFormatInfo> = PREVIEW_FORMATS
        .iter()
        .copied()
        .map(ShellPreviewFormat::info)
        .collect();
    let common: Vec<&ShellPreviewFormatInfo> =
        formats.iter().filter(|format| format.common).collect();

    assert!(common.len() > 10);
    assert!(common.len() < formats.len());
    assert!(common.iter().any(|format| format.extension == "mp4"));
    assert!(common.iter().any(|format| format.extension == "mkv"));
    assert!(common.iter().any(|format| format.extension == "mp3"));
    assert!(common.iter().any(|format| format.extension == "wav"));
    assert!(common.iter().any(|format| format.extension == "m4b"));
    assert!(!common.iter().any(|format| format.extension == "mxf"));
    assert!(!common.iter().any(|format| format.extension == "wv"));
}

#[test]
fn selected_preview_registration_filters_to_requested_formats() {
    let formats =
        filter_preview_formats(&[".MP4".to_string(), "mkv".to_string(), "wv".to_string()])
            .expect("selected formats should be accepted");
    let extensions: Vec<&str> = formats.iter().map(|format| format.extension).collect();

    assert_eq!(extensions, vec!["mkv", "mp4", "wv"]);
}

#[test]
fn selected_preview_registration_rejects_empty_or_unknown_formats() {
    assert!(filter_preview_formats(&[]).is_err());
    assert!(filter_preview_formats(&["unknown".to_string()]).is_err());
}

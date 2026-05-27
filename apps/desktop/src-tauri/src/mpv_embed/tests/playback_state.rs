use super::*;

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
fn startup_snapshots_use_known_state_without_live_mpv_property_reads() {
    let snapshot = startup_snapshot_for_interactive_open(
        "https://example.com/live.m3u8",
        42,
        37.5,
        false,
        "playing",
    );

    assert_eq!(snapshot.path, "https://example.com/live.m3u8");
    assert_eq!(snapshot.hwnd, 42);
    assert_eq!(snapshot.status, "playing");
    assert!(!snapshot.paused);
    assert!(!snapshot.ended);
    assert_eq!(snapshot.position, 0.0);
    assert_eq!(snapshot.duration, 0.0);
    assert_eq!(snapshot.fps, 0.0);
    assert_eq!(snapshot.speed, 1.0);
    assert_eq!(snapshot.hwdec, "auto-safe");
    assert_eq!(snapshot.volume, 37.5);
    assert!(snapshot.tracks.is_empty());
}

#[test]
fn startup_snapshots_preserve_requested_pause_status() {
    let snapshot = startup_snapshot_for_interactive_open(
        "rtsp://camera.local/stream",
        0,
        80.0,
        true,
        "paused",
    );

    assert_eq!(snapshot.status, "paused");
    assert!(snapshot.paused);
    assert!(snapshot.video_fill);
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

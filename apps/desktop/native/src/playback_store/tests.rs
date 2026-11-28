use super::helpers::*;
use super::*;
use std::fs;

#[test]
fn resume_position_uses_duration_ratios() {
    assert_eq!(resume_position_for_entry(0.5, 2.0), 0.5);
    assert_eq!(resume_position_for_entry(2.0, 400.0), 0.0);
    assert_eq!(resume_position_for_entry(96.0, 100.0), 0.0);
}

#[test]
fn redb_store_updates_existing_paths_and_lists_newest_first() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-history-{}-{}",
        std::process::id(),
        now_millis()
    ));
    fs::create_dir_all(&directory).expect("temp history directory should be created");
    let database_path = directory.join("history.redb");
    let mut store = PlaybackStore::open(database_path).expect("redb store should open");

    store
        .remember(PlaybackHistoryUpdate {
            path: "E:\\Media\\first.mp4".to_string(),
            name: Some("first.mp4".to_string()),
            position: 40.0,
            duration: 100.0,
            updated_at: Some(10),
        })
        .expect("first entry should be written");
    store
        .remember(PlaybackHistoryUpdate {
            path: "E:\\Media\\second.mp4".to_string(),
            name: Some("second.mp4".to_string()),
            position: 80.0,
            duration: 100.0,
            updated_at: Some(20),
        })
        .expect("second entry should be written");
    store
        .remember(PlaybackHistoryUpdate {
            path: "E:\\Media\\first.mp4".to_string(),
            name: Some("first.mp4".to_string()),
            position: 50.0,
            duration: 100.0,
            updated_at: Some(30),
        })
        .expect("first entry should be updated");

    let entries = store.list().expect("history should be readable");
    let _ = fs::remove_dir_all(&directory);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, "E:\\Media\\first.mp4");
    assert_eq!(entries[0].position, 50.0);
    assert_eq!(entries[1].path, "E:\\Media\\second.mp4");
}

#[test]
fn redb_store_clears_history_and_resume_positions() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-history-clear-{}-{}",
        std::process::id(),
        now_millis()
    ));
    fs::create_dir_all(&directory).expect("temp history directory should be created");
    let database_path = directory.join("history.redb");
    let mut store = PlaybackStore::open(database_path).expect("redb store should open");

    store
        .remember(PlaybackHistoryUpdate {
            path: "E:\\Media\\first.mp4".to_string(),
            name: Some("first.mp4".to_string()),
            position: 40.0,
            duration: 100.0,
            updated_at: Some(10),
        })
        .expect("entry should be written");

    let entries = store.clear().expect("history should clear");
    let resume = store
        .resume_position("E:\\Media\\first.mp4")
        .expect("resume lookup should still work");
    let _ = fs::remove_dir_all(&directory);

    assert!(entries.is_empty());
    assert_eq!(resume, 0.0);
}

#[test]
fn redb_store_matches_windows_history_paths_case_insensitively() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-history-windows-key-{}-{}",
        std::process::id(),
        now_millis()
    ));
    fs::create_dir_all(&directory).expect("temp history directory should be created");
    let database_path = directory.join("history.redb");
    let mut store = PlaybackStore::open(database_path).expect("redb store should open");

    store
        .remember(PlaybackHistoryUpdate {
            path: "F:\\PP\\292MY-1051\\hhd800.com@292MY-1051.mp4".to_string(),
            name: None,
            position: 120.0,
            duration: 600.0,
            updated_at: Some(10),
        })
        .expect("entry should be written");

    let resume = store
        .resume_position("\\\\?\\f:\\pp\\292my-1051\\hhd800.com@292my-1051.mp4")
        .expect("resume lookup should normalize Windows paths");
    let _ = fs::remove_dir_all(&directory);

    assert_eq!(resume, 120.0);
}

#[test]
fn redb_store_persists_global_and_media_playback_settings() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-playback-settings-{}-{}",
        std::process::id(),
        now_millis()
    ));
    fs::create_dir_all(&directory).expect("temp settings directory should be created");
    let database_path = directory.join("history.redb");
    let mut store = PlaybackStore::open(database_path).expect("redb store should open");

    store
        .update_settings(PlaybackSettingsUpdate {
            volume: Some(64.0),
            loop_mode: Some("all".to_string()),
            hwdec_mode: Some("software".to_string()),
            playback_speed: Some(1.25),
            video_fill: Some(true),
            time_display_mode: Some("frames".to_string()),
        })
        .expect("settings should be written");
    store
        .update_media_settings(
            "F:\\PP\\292MY-1051\\hhd800.com@292MY-1051.mp4",
            MediaPlaybackSettingsUpdate {
                subtitle_track_id: Some(Some(3)),
            },
        )
        .expect("media settings should be written");

    drop(store);
    let store =
        PlaybackStore::open(directory.join("history.redb")).expect("redb store should reopen");
    let settings = store.settings().expect("settings should be readable");
    let media_settings = store
        .media_settings("\\\\?\\f:\\pp\\292my-1051\\hhd800.com@292my-1051.mp4")
        .expect("media settings should be readable");
    let _ = fs::remove_dir_all(&directory);

    assert_eq!(settings.volume, 64.0);
    assert_eq!(settings.loop_mode, "all");
    assert_eq!(settings.hwdec_mode, "software");
    assert_eq!(settings.playback_speed, 1.25);
    assert!(settings.video_fill);
    assert_eq!(settings.time_display_mode, "frames");
    assert_eq!(media_settings.subtitle_track_id, Some(3));
}

#[test]
fn redb_store_persists_network_stream_history_newest_first() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-network-streams-{}-{}",
        std::process::id(),
        now_millis()
    ));
    fs::create_dir_all(&directory).expect("temp stream directory should be created");
    let database_path = directory.join("history.redb");
    let mut store = PlaybackStore::open(database_path).expect("redb store should open");

    store
        .remember_network_stream(NetworkStreamHistoryUpdate {
            url: "rtsp://camera.local/live".to_string(),
            name: None,
            updated_at: Some(10),
        })
        .expect("rtsp stream should be stored");
    store
        .remember_network_stream(NetworkStreamHistoryUpdate {
            url: "https://example.com/live/channel.m3u8".to_string(),
            name: Some("Example Live".to_string()),
            updated_at: Some(20),
        })
        .expect("https stream should be stored");
    store
        .remember_network_stream(NetworkStreamHistoryUpdate {
            url: "RTSP://camera.local/live".to_string(),
            name: Some("Front Door".to_string()),
            updated_at: Some(30),
        })
        .expect("same rtsp stream should update after protocol normalization");

    let entries = store
        .network_stream_history()
        .expect("network stream history should be readable");
    let _ = fs::remove_dir_all(&directory);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].url, "rtsp://camera.local/live");
    assert_eq!(entries[0].name, "Front Door");
    assert_eq!(entries[0].scheme, "rtsp");
    assert_eq!(entries[1].url, "https://example.com/live/channel.m3u8");
    assert_eq!(entries[1].name, "Example Live");
}

#[test]
fn redb_store_clears_network_stream_history() {
    let directory = std::env::temp_dir().join(format!(
        "openplayer-network-streams-clear-{}-{}",
        std::process::id(),
        now_millis()
    ));
    fs::create_dir_all(&directory).expect("temp stream directory should be created");
    let database_path = directory.join("history.redb");
    let mut store = PlaybackStore::open(database_path).expect("redb store should open");

    store
        .remember_network_stream(NetworkStreamHistoryUpdate {
            url: "rtmp://example.com/live".to_string(),
            name: None,
            updated_at: Some(10),
        })
        .expect("rtmp stream should be stored");

    let entries = store
        .clear_network_stream_history()
        .expect("network stream history should clear");
    let after_clear = store
        .network_stream_history()
        .expect("network stream history should be readable");
    let _ = fs::remove_dir_all(&directory);

    assert!(entries.is_empty());
    assert!(after_clear.is_empty());
}

#[test]
fn network_stream_history_rejects_unsupported_protocols() {
    let error = normalize_network_stream_update(NetworkStreamHistoryUpdate {
        url: "file:///C:/secret.mp4".to_string(),
        name: None,
        updated_at: Some(10),
    })
    .expect_err("local file urls should not be accepted as network streams");

    assert!(error.contains("unsupported network stream protocol"));
}

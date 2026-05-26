use super::*;

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

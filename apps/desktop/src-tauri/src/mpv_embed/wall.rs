#[cfg(windows)]
use super::video_host::window_hwnd;
use super::*;

#[cfg(windows)]
pub(super) fn wall_open_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let normalized = normalize_wall_tile_requests(tiles)?;
    if state.can_reuse_open_wall(&normalized)? {
        let generation = state.current_generation()?;
        start_missing_wall_tiles(app, state, generation, &normalized)?;
        return wall_layout_for_app(
            app,
            state,
            normalized
                .iter()
                .map(|tile| MpvWallTileLayout {
                    id: tile.id.clone(),
                    x: tile.rect.x,
                    y: tile.rect.y,
                    width: tile.rect.width,
                    height: tile.rect.height,
                })
                .collect(),
        );
    }

    let generation = state.next_generation()?;
    let old_players = state.take_players()?;
    destroy_wall_players_on_main(app, old_players)?;
    let snapshots = wall_initial_snapshots(&normalized);
    state.replace_opening_state(snapshots.clone())?;

    start_missing_wall_tiles(app, state, generation, &normalized)?;

    Ok(snapshots)
}

#[cfg(not(windows))]
pub(super) fn wall_open_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let _ = (app, state, tiles);
    Err("native multi-stream wall currently supports Windows".to_string())
}

#[cfg(windows)]
pub(super) fn wall_layout_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let layouts = normalize_wall_tile_layouts(tiles)?;
    let host_window = wall_host_window(app)?;
    let mut host_layouts = Vec::new();
    let mut players = state
        .players
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;

    for layout in layouts {
        if let Some(player) = players.get_mut(&layout.id) {
            let host_layout = wall_tile_layout_for_window(&host_window, layout.rect)?;
            player.rect = layout.rect;
            host_layouts.push(MpvWallHostLayout {
                id: layout.id,
                layout: host_layout,
            });
        }
    }
    drop(players);

    schedule_wall_video_hosts_resize_on_main(app, host_layouts)?;
    wall_snapshot(state)
}

#[cfg(not(windows))]
pub(super) fn wall_layout_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let _ = (app, state, tiles);
    Err("native multi-stream wall currently supports Windows".to_string())
}

#[cfg(windows)]
pub(super) fn wall_snapshot(state: &MpvWallState) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let players = state
        .players
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    let mut statuses = state
        .statuses
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    for (id, player) in players.iter() {
        statuses.insert(id.clone(), player.live_snapshot());
    }
    Ok(statuses.values().cloned().collect())
}

#[cfg(not(windows))]
pub(super) fn wall_snapshot(state: &MpvWallState) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let _ = state;
    Ok(Vec::new())
}

#[cfg(windows)]
pub(super) fn wall_close(app: &AppHandle, state: &MpvWallState) -> Result<(), String> {
    let _ = state.next_generation()?;
    let players = state.take_players()?;
    let mut starting = state
        .starting
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    starting.clear();
    drop(starting);
    let mut statuses = state
        .statuses
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    statuses.clear();
    drop(statuses);
    destroy_wall_players_on_main(app, players)
}

#[cfg(windows)]
pub(super) fn wall_set_visible(
    app: &AppHandle,
    state: &MpvWallState,
    visible: bool,
) -> Result<(), String> {
    let _ = state;
    let app_for_main = app.clone();
    app.run_on_main_thread(move || {
        let state = app_for_main.state::<MpvWallState>();
        if let Ok(players) = state.players.lock() {
            for player in players.values() {
                player.host.set_visible(visible);
            }
        }
    })
    .map_err(|error| format!("failed to schedule mpv wall visibility update: {error}"))
}

#[cfg(not(windows))]
pub(super) fn wall_close(app: &AppHandle, state: &MpvWallState) -> Result<(), String> {
    let _ = (app, state);
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn wall_set_visible(
    app: &AppHandle,
    state: &MpvWallState,
    visible: bool,
) -> Result<(), String> {
    let _ = (app, state, visible);
    Ok(())
}

#[cfg(windows)]
impl MpvWallState {
    pub(super) fn next_generation(&self) -> Result<u64, String> {
        let mut generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        *generation = generation.saturating_add(1);
        Ok(*generation)
    }

    pub(super) fn current_generation(&self) -> Result<u64, String> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(*generation)
    }

    pub(super) fn is_generation_current(&self, expected: u64) -> Result<bool, String> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(*generation == expected)
    }

    pub(super) fn can_reuse_open_wall(
        &self,
        tiles: &[NormalizedMpvWallTileRequest],
    ) -> Result<bool, String> {
        if tiles.is_empty() {
            return Ok(false);
        }
        let statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        if statuses.len() != tiles.len() {
            return Ok(false);
        }

        Ok(tiles.iter().all(|tile| {
            statuses
                .get(&tile.id)
                .is_some_and(|snapshot| snapshot.url == tile.url)
        }))
    }

    pub(super) fn replace_opening_state(
        &self,
        snapshots: Vec<MpvWallTileSnapshot>,
    ) -> Result<(), String> {
        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        starting.clear();
        drop(starting);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        *statuses = snapshots
            .into_iter()
            .map(|snapshot| (snapshot.id.clone(), snapshot))
            .collect();
        Ok(())
    }

    pub(super) fn take_players(&self) -> Result<BTreeMap<String, MpvWallPlayer>, String> {
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(std::mem::take(&mut *players))
    }

    pub(super) fn insert_player(
        &self,
        generation: u64,
        player: MpvWallPlayer,
        status: &str,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let snapshot = player.status_snapshot(status, None);
        let id = player.id.clone();
        let _ = self.clear_tile_starting(&id);
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        players.insert(id.clone(), player);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(id, snapshot);
        Ok(())
    }

    pub(super) fn mark_tile_starting(&self, generation: u64, id: &str) -> Result<bool, String> {
        if !self.is_generation_current(generation)? {
            return Ok(false);
        }
        let players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        if players.contains_key(id) {
            return Ok(false);
        }
        drop(players);

        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(starting.insert(id.to_string()))
    }

    pub(super) fn clear_tile_starting(&self, id: &str) -> Result<(), String> {
        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        starting.remove(id);
        Ok(())
    }

    pub(super) fn update_player_status(
        &self,
        generation: u64,
        id: &str,
        status: &str,
        message: Option<String>,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        let Some(player) = players.get(id) else {
            return Ok(());
        };
        let snapshot = player.status_snapshot(status, message);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(id.to_string(), snapshot);
        Ok(())
    }

    pub(super) fn update_tile_error(
        &self,
        generation: u64,
        tile: &NormalizedMpvWallTileRequest,
        message: String,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let _ = self.clear_tile_starting(&tile.id);
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        players.remove(&tile.id);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(
            tile.id.clone(),
            wall_tile_status_snapshot(tile, "error", Some(message)),
        );
        Ok(())
    }
}

#[cfg(windows)]
pub(super) fn start_missing_wall_tiles(
    app: &AppHandle,
    state: &MpvWallState,
    generation: u64,
    tiles: &[NormalizedMpvWallTileRequest],
) -> Result<(), String> {
    for (index, tile) in tiles.iter().cloned().enumerate() {
        if state.mark_tile_starting(generation, &tile.id)? {
            spawn_wall_tile_start(
                app.clone(),
                generation,
                wall_request_id(generation, index),
                tile,
                MPV_WALL_TILE_START_STAGGER.saturating_mul(index as u32),
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn spawn_wall_tile_start(
    app: AppHandle,
    generation: u64,
    request_id: u64,
    tile: NormalizedMpvWallTileRequest,
    delay: Duration,
) {
    let _ = thread::Builder::new()
        .name(format!("openplayer-wall-{}", tile.id))
        .spawn(move || {
            if !delay.is_zero() {
                thread::sleep(delay);
            }
            let state = app.state::<MpvWallState>();
            if let Err(error) =
                wall_start_tile_for_app(&app, state.inner(), generation, request_id, &tile)
            {
                let _ = state
                    .inner()
                    .update_tile_error(generation, &tile, error.to_string());
            }
        });
}

#[cfg(windows)]
pub(super) fn wall_start_tile_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    generation: u64,
    request_id: u64,
    tile: &NormalizedMpvWallTileRequest,
) -> Result<(), String> {
    if !state.is_generation_current(generation)? {
        return Ok(());
    }

    let host = create_wall_video_host_on_main(app, tile.rect)?;
    let mpv = Arc::new(create_embed_player_without_logs(host.wid())?);
    configure_wall_osd(mpv.as_ref());
    if tile.muted {
        mpv.set_property("volume", 0.0)
            .map_err(|error| format!("mpv wall mute failed: {error}"))?;
    }
    state.insert_player(
        generation,
        MpvWallPlayer {
            id: tile.id.clone(),
            url: tile.url.clone(),
            title: tile.title.clone(),
            rect: tile.rect,
            mpv: Arc::clone(&mpv),
            host,
        },
        "loading",
    )?;

    if !state.is_generation_current(generation)? {
        return Ok(());
    }

    load_media_file_async(mpv.as_ref(), &tile.url, None, request_id)?;
    state.update_player_status(generation, &tile.id, "playing", None)
}

pub(super) fn wall_request_id(generation: u64, index: usize) -> u64 {
    generation
        .saturating_mul(1_000)
        .saturating_add(index as u64)
        .saturating_add(1)
}

#[cfg(windows)]
pub(super) fn create_wall_video_host_on_main(
    app: &AppHandle,
    rect: MpvWallTileRect,
) -> Result<MpvVideoHost, String> {
    let host_window = wall_host_window(app)?;
    let layout = wall_tile_layout_for_window(&host_window, rect)?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let result =
            MpvVideoHost::new_with_layout(&host_window, layout, MPV_WALL_TILE_CORNER_RADIUS);
        let _ = sender.send(result);
    })
    .map_err(|error| format!("failed to schedule mpv wall host creation: {error}"))?;

    receiver
        .recv()
        .map_err(|_| "mpv wall host creation did not return a result".to_string())?
}

#[cfg(windows)]
pub(super) fn destroy_wall_players_on_main(
    app: &AppHandle,
    players: BTreeMap<String, MpvWallPlayer>,
) -> Result<(), String> {
    if players.is_empty() {
        return Ok(());
    }

    let mut hosts = Vec::with_capacity(players.len());
    for (_, player) in players {
        let MpvWallPlayer { mpv, host, .. } = player;
        let _ = mpv.command("stop", &[]);
        hosts.push(host);
    }

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        for host in &mut hosts {
            host.destroy();
        }
        let _ = sender.send(());
    })
    .map_err(|error| format!("failed to schedule mpv wall host teardown: {error}"))?;

    receiver
        .recv()
        .map_err(|_| "mpv wall host teardown did not return a result".to_string())
}

#[cfg(windows)]
pub(super) fn schedule_wall_video_hosts_resize_on_main(
    app: &AppHandle,
    layouts: Vec<MpvWallHostLayout>,
) -> Result<(), String> {
    if layouts.is_empty() {
        return Ok(());
    }

    let app_for_main = app.clone();
    app.run_on_main_thread(move || {
        let state = app_for_main.state::<MpvWallState>();
        if let Ok(mut players) = state.players.lock() {
            for host_layout in layouts {
                if let Some(player) = players.get_mut(&host_layout.id) {
                    let _ = player.host.resize_to_layout(host_layout.layout);
                }
            }
        }
    })
    .map_err(|error| format!("failed to schedule mpv wall host resize: {error}"))
}

#[cfg(windows)]
pub(super) fn wall_host_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("overlay")
        .or_else(|| app.get_webview_window("main"))
        .ok_or_else(|| "mpv wall host window is unavailable".to_string())
}

pub(super) fn wall_initial_snapshots(
    tiles: &[NormalizedMpvWallTileRequest],
) -> Vec<MpvWallTileSnapshot> {
    tiles
        .iter()
        .map(|tile| wall_tile_status_snapshot(tile, "loading", None))
        .collect()
}

pub(super) fn wall_tile_status_snapshot(
    tile: &NormalizedMpvWallTileRequest,
    status: &str,
    message: Option<String>,
) -> MpvWallTileSnapshot {
    MpvWallTileSnapshot {
        id: tile.id.clone(),
        url: tile.url.clone(),
        title: tile.title.clone(),
        status: status.to_string(),
        latency_seconds: None,
        buffer_seconds: None,
        bitrate_bps: None,
        message,
    }
}

#[cfg(windows)]
impl MpvWallPlayer {
    pub(super) fn live_snapshot(&self) -> MpvWallTileSnapshot {
        wall_player_snapshot(self)
    }

    pub(super) fn status_snapshot(
        &self,
        status: &str,
        message: Option<String>,
    ) -> MpvWallTileSnapshot {
        MpvWallTileSnapshot {
            id: self.id.clone(),
            url: self.url.clone(),
            title: self.title.clone(),
            status: status.to_string(),
            latency_seconds: None,
            buffer_seconds: None,
            bitrate_bps: None,
            message,
        }
    }
}

#[cfg(any(windows, test))]
pub(super) fn wall_live_status(eof_reached: bool, paused: bool, idle: bool) -> &'static str {
    if eof_reached {
        "ended"
    } else if paused {
        "paused"
    } else if idle {
        "loading"
    } else {
        "playing"
    }
}

#[cfg(any(windows, test))]
pub(super) fn combine_wall_bitrate(
    video_bitrate: Option<f64>,
    audio_bitrate: Option<f64>,
    raw_input_bytes_per_second: Option<f64>,
) -> Option<f64> {
    let track_bitrate = video_bitrate
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(0.0)
        + audio_bitrate
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(0.0);
    if track_bitrate > 0.0 {
        return Some(track_bitrate);
    }

    raw_input_bytes_per_second
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|bytes_per_second| bytes_per_second * 8.0)
}

#[cfg(windows)]
pub(super) fn read_finite_mpv_property(mpv: &libmpv2::Mpv, property: &str) -> Option<f64> {
    mpv.get_property::<f64>(property)
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .or_else(|| {
            mpv.get_property::<i64>(property)
                .ok()
                .map(|value| value as f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
        })
}

#[cfg(windows)]
pub(super) fn read_wall_buffer(mpv: &libmpv2::Mpv) -> Option<f64> {
    read_finite_mpv_property(mpv, "demuxer-cache-duration")
        .or_else(|| read_finite_mpv_property(mpv, "demuxer-cache-state/cache-duration"))
        .or_else(|| read_finite_mpv_property(mpv, "cache-duration"))
        .or_else(|| {
            let cache_time = read_finite_mpv_property(mpv, "demuxer-cache-time")?;
            let position = read_finite_mpv_property(mpv, "time-pos")?;
            let buffered = cache_time - position;
            (buffered.is_finite() && buffered >= 0.0).then_some(buffered)
        })
}

#[cfg(windows)]
pub(super) fn read_wall_bitrate(mpv: &libmpv2::Mpv) -> Option<f64> {
    combine_wall_bitrate(
        read_finite_mpv_property(mpv, "video-bitrate"),
        read_finite_mpv_property(mpv, "audio-bitrate"),
        read_finite_mpv_property(mpv, "cache-speed")
            .or_else(|| read_finite_mpv_property(mpv, "demuxer-cache-state/raw-input-rate")),
    )
}

#[cfg(windows)]
pub(super) fn configure_wall_osd(mpv: &libmpv2::Mpv) {
    let _ = mpv.set_property("osd-align-x", "left");
    let _ = mpv.set_property("osd-align-y", "top");
    let _ = mpv.set_property("osd-margin-x", 12);
    let _ = mpv.set_property("osd-margin-y", 12);
    let _ = mpv.set_property("osd-font-size", 18);
    let _ = mpv.set_property("osd-bold", true);
    let _ = mpv.set_property("osd-color", "#f1c66b");
    let _ = mpv.set_property("osd-border-color", "#120f08");
    let _ = mpv.set_property("osd-border-size", 1.8);
    let _ = mpv.set_property("osd-shadow-color", "#000000");
    let _ = mpv.set_property("osd-shadow-offset", 1.0);
    let _ = mpv.set_property("osd-back-color", "#99000000");
}

#[cfg(any(windows, test))]
pub(super) fn format_wall_buffer_millis(buffer_seconds: Option<f64>) -> String {
    buffer_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| format!("{} ms", (value * 1000.0).round() as i64))
        .unwrap_or_else(|| "-- ms".to_string())
}

#[cfg(any(windows, test))]
pub(super) fn format_wall_bitrate(bits_per_second: Option<f64>) -> String {
    let Some(bits_per_second) = bits_per_second.filter(|value| value.is_finite() && *value > 0.0)
    else {
        return "--".to_string();
    };
    if bits_per_second >= 1_000_000.0 {
        format!("{:.1} Mbps", bits_per_second / 1_000_000.0)
    } else {
        format!("{} Kbps", (bits_per_second / 1000.0).round() as i64)
    }
}

#[cfg(windows)]
pub(super) fn update_wall_osd(
    mpv: &libmpv2::Mpv,
    buffer_seconds: Option<f64>,
    bitrate_bps: Option<f64>,
) {
    let text = format!(
        "BUF {} · {}",
        format_wall_buffer_millis(buffer_seconds),
        format_wall_bitrate(bitrate_bps)
    );
    let _ = mpv.command("show-text", &[text.as_str(), "1500", "1"]);
}

#[cfg(windows)]
pub(super) fn drain_wall_player_events(mpv: &libmpv2::Mpv) {
    for _ in 0..MPV_WALL_EVENT_DRAIN_LIMIT {
        let Some(event) = mpv.wait_event(0.0) else {
            break;
        };
        let _ = handle_mpv_event(event);
    }
}

#[cfg(windows)]
pub(super) fn read_wall_bool_property(mpv: &libmpv2::Mpv, property: &str) -> bool {
    mpv.get_property::<bool>(property).unwrap_or(false)
}

#[cfg(windows)]
pub(super) fn wall_player_snapshot(player: &MpvWallPlayer) -> MpvWallTileSnapshot {
    drain_wall_player_events(player.mpv.as_ref());
    let status = wall_live_status(
        read_wall_bool_property(player.mpv.as_ref(), "eof-reached"),
        read_wall_bool_property(player.mpv.as_ref(), "pause"),
        read_wall_bool_property(player.mpv.as_ref(), "idle-active"),
    );
    let buffer_seconds = read_wall_buffer(player.mpv.as_ref());
    let bitrate_bps = read_wall_bitrate(player.mpv.as_ref());
    update_wall_osd(player.mpv.as_ref(), buffer_seconds, bitrate_bps);

    MpvWallTileSnapshot {
        id: player.id.clone(),
        url: player.url.clone(),
        title: player.title.clone(),
        status: status.to_string(),
        latency_seconds: None,
        buffer_seconds,
        bitrate_bps,
        message: None,
    }
}

pub(super) fn normalize_wall_tile_requests(
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<NormalizedMpvWallTileRequest>, String> {
    if tiles.is_empty() {
        return Ok(Vec::new());
    }
    if tiles.len() > MAX_MPV_WALL_TILES {
        return Err(format!(
            "mpv wall supports at most {MAX_MPV_WALL_TILES} streams"
        ));
    }

    let mut ids = BTreeMap::new();
    let mut normalized = Vec::with_capacity(tiles.len());
    for tile in tiles {
        let tile = normalize_wall_tile_request(tile)?;
        if ids.insert(tile.id.clone(), ()).is_some() {
            return Err(format!("duplicate mpv wall tile id: {}", tile.id));
        }
        normalized.push(tile);
    }
    Ok(normalized)
}

pub(super) fn normalize_wall_tile_request(
    tile: MpvWallTileRequest,
) -> Result<NormalizedMpvWallTileRequest, String> {
    let id = normalize_wall_tile_id(&tile.id)?;
    let url = validate_media_path(&tile.url)?
        .to_string_lossy()
        .to_string();
    let title = tile
        .title
        .map(|title| title.trim().chars().take(128).collect::<String>())
        .filter(|title| !title.is_empty());
    Ok(NormalizedMpvWallTileRequest {
        id,
        url,
        title,
        rect: normalize_wall_tile_rect(tile.x, tile.y, tile.width, tile.height)?,
        muted: tile.muted.unwrap_or(true),
    })
}

pub(super) fn normalize_wall_tile_layouts(
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<NormalizedMpvWallTileLayout>, String> {
    if tiles.len() > MAX_MPV_WALL_TILES {
        return Err(format!(
            "mpv wall supports at most {MAX_MPV_WALL_TILES} layout items"
        ));
    }
    tiles
        .into_iter()
        .map(|tile| {
            Ok(NormalizedMpvWallTileLayout {
                id: normalize_wall_tile_id(&tile.id)?,
                rect: normalize_wall_tile_rect(tile.x, tile.y, tile.width, tile.height)?,
            })
        })
        .collect()
}

pub(super) fn normalize_wall_tile_id(id: &str) -> Result<String, String> {
    let id = id.trim();
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | '-'))
    {
        return Err(format!("mpv wall tile id is invalid: {id}"));
    }
    Ok(id.to_string())
}

pub(super) fn normalize_wall_tile_rect(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<MpvWallTileRect, String> {
    if ![x, y, width, height].into_iter().all(f64::is_finite) {
        return Err("mpv wall tile layout must use finite numbers".to_string());
    }
    let x = x.clamp(0.0, 1.0 - MIN_MPV_WALL_TILE_RATIO);
    let y = y.clamp(0.0, 1.0 - MIN_MPV_WALL_TILE_RATIO);
    let width = width.clamp(MIN_MPV_WALL_TILE_RATIO, 1.0 - x);
    let height = height.clamp(MIN_MPV_WALL_TILE_RATIO, 1.0 - y);
    Ok(MpvWallTileRect {
        x,
        y,
        width,
        height,
    })
}

pub(super) fn wall_tile_rect_to_video_host_rect(
    parent_width: i32,
    parent_height: i32,
    rect: MpvWallTileRect,
) -> VideoHostRect {
    let parent_width = parent_width.max(1);
    let parent_height = parent_height.max(1);
    let x = (rect.x * f64::from(parent_width)).round() as i32;
    let y = (rect.y * f64::from(parent_height)).round() as i32;
    let max_width = parent_width.saturating_sub(x).max(1);
    let max_height = parent_height.saturating_sub(y).max(1);
    let width = ((rect.width * f64::from(parent_width)).round() as i32)
        .max(1)
        .min(max_width);
    let height = ((rect.height * f64::from(parent_height)).round() as i32)
        .max(1)
        .min(max_height);
    VideoHostRect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(windows)]
pub(super) fn wall_tile_layout_for_window(
    window: &impl HasWindowHandle,
    rect: MpvWallTileRect,
) -> Result<VideoHostRect, String> {
    let parent_hwnd = window_hwnd(window)?;
    let parent = parent_hwnd as isize as HWND;
    let mut client = RECT::default();
    if unsafe { GetClientRect(parent, &mut client) } == 0 {
        return Err("failed to read window size for mpv wall tile".to_string());
    }
    Ok(inset_wall_video_host_rect(
        wall_tile_rect_to_video_host_rect(
            client.right - client.left,
            client.bottom - client.top,
            rect,
        ),
    ))
}

#[cfg(windows)]
pub(super) fn inset_wall_video_host_rect(layout: VideoHostRect) -> VideoHostRect {
    let inset = MPV_WALL_TILE_BORDER_INSET
        .min(layout.width / 3)
        .min(layout.height / 3);
    if inset <= 0 {
        return layout;
    }
    VideoHostRect {
        x: layout.x + inset,
        y: layout.y + inset,
        width: (layout.width - inset.saturating_mul(2)).max(1),
        height: (layout.height - inset.saturating_mul(2)).max(1),
    }
}

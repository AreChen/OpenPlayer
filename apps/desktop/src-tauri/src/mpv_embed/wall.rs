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

#[cfg(not(windows))]
pub(super) fn wall_close(app: &AppHandle, state: &MpvWallState) -> Result<(), String> {
    let _ = (app, state);
    Ok(())
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
pub(super) fn wall_set_visible(
    app: &AppHandle,
    state: &MpvWallState,
    visible: bool,
) -> Result<(), String> {
    let _ = (app, state, visible);
    Ok(())
}

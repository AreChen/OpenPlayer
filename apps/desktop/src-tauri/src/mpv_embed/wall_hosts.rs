use super::*;

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

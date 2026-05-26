use super::*;

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

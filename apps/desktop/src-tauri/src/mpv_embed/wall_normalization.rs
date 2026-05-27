#[cfg(windows)]
use super::video_host::window_hwnd;
use super::*;

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
        playback: normalize_wall_playback_options(tile.playback.unwrap_or_default()),
    })
}

pub(super) fn normalize_wall_playback_options(
    options: MpvWallPlaybackOptions,
) -> MpvWallPlaybackOptions {
    MpvWallPlaybackOptions {
        latency_mode: options.latency_mode,
        rtsp_transport: options.rtsp_transport,
        buffer_ms: options.buffer_ms.map(|value| value.clamp(50, 2_000)),
    }
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

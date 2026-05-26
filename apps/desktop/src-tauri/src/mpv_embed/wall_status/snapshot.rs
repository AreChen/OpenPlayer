use super::super::*;
#[cfg(windows)]
use super::metrics::{read_wall_bitrate, read_wall_bool_property, read_wall_buffer};
#[cfg(windows)]
use super::osd::update_wall_osd;

pub(in crate::mpv_embed) fn wall_initial_snapshots(
    tiles: &[NormalizedMpvWallTileRequest],
) -> Vec<MpvWallTileSnapshot> {
    tiles
        .iter()
        .map(|tile| wall_tile_status_snapshot(tile, "loading", None))
        .collect()
}

pub(in crate::mpv_embed) fn wall_tile_status_snapshot(
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
    pub(in crate::mpv_embed) fn live_snapshot(&self) -> MpvWallTileSnapshot {
        wall_player_snapshot(self)
    }

    pub(in crate::mpv_embed) fn status_snapshot(
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
pub(in crate::mpv_embed) fn wall_live_status(
    eof_reached: bool,
    paused: bool,
    idle: bool,
) -> &'static str {
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

#[cfg(windows)]
pub(in crate::mpv_embed) fn drain_wall_player_events(mpv: &libmpv2::Mpv) {
    for _ in 0..MPV_WALL_EVENT_DRAIN_LIMIT {
        let Some(event) = mpv.wait_event(0.0) else {
            break;
        };
        let _ = handle_mpv_event(event);
    }
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn wall_player_snapshot(player: &MpvWallPlayer) -> MpvWallTileSnapshot {
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

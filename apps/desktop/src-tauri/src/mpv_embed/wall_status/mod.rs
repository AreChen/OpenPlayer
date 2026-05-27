mod metrics;
mod osd;
mod snapshot;

#[cfg(test)]
pub(in crate::mpv_embed) use metrics::combine_wall_bitrate;
#[cfg(windows)]
pub(in crate::mpv_embed) use osd::configure_wall_osd;
#[cfg(test)]
pub(in crate::mpv_embed) use osd::format_wall_transport_latency;
#[cfg(test)]
pub(in crate::mpv_embed) use osd::{format_wall_bitrate, format_wall_buffer_millis};
#[cfg(any(windows, test))]
pub(in crate::mpv_embed) use snapshot::wall_initial_snapshots;
#[cfg(test)]
pub(in crate::mpv_embed) use snapshot::wall_live_status;
#[cfg(windows)]
pub(in crate::mpv_embed) use snapshot::wall_tile_status_snapshot;

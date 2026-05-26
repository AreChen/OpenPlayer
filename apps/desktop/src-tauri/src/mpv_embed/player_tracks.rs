use super::*;

pub(super) fn valid_fps(value: f64) -> Option<f64> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

pub(super) fn read_player_fps(mpv: &libmpv2::Mpv) -> f64 {
    mpv.get_property::<f64>("container-fps")
        .ok()
        .and_then(valid_fps)
        .or_else(|| {
            mpv.get_property::<f64>("estimated-vf-fps")
                .ok()
                .and_then(valid_fps)
        })
        .unwrap_or(0.0)
}

pub(super) fn read_optional_string(mpv: &libmpv2::Mpv, property: &str) -> Option<String> {
    mpv.get_property::<String>(property)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn read_tracks(mpv: &libmpv2::Mpv) -> Vec<MpvEmbedTrack> {
    let count = mpv
        .get_property::<i64>("track-list/count")
        .unwrap_or(0)
        .clamp(0, MAX_TRACKS);
    let mut tracks = Vec::new();

    for index in 0..count {
        let id = match mpv.get_property::<i64>(&format!("track-list/{index}/id")) {
            Ok(value) if value > 0 => value,
            _ => continue,
        };
        let kind = match mpv.get_property::<String>(&format!("track-list/{index}/type")) {
            Ok(value) if matches!(value.as_str(), "audio" | "video" | "sub") => value,
            _ => continue,
        };

        tracks.push(MpvEmbedTrack {
            id,
            kind,
            title: read_optional_string(mpv, &format!("track-list/{index}/title")),
            language: read_optional_string(mpv, &format!("track-list/{index}/lang")),
            codec: read_optional_string(mpv, &format!("track-list/{index}/codec")),
            selected: mpv
                .get_property::<bool>(&format!("track-list/{index}/selected"))
                .unwrap_or(false),
            external: mpv
                .get_property::<bool>(&format!("track-list/{index}/external"))
                .unwrap_or(false),
        });
    }

    tracks
}

use std::{collections::HashMap, path::Path};

use libmpv2::Mpv;

use crate::media_paths::{discover_sidecar_subtitles, is_hls_manifest_media_url};

use super::Media;

pub(super) fn loadfile(mpv: &Mpv, media: &Media, resume: f64) -> libmpv2::Result<()> {
    let options = if media.network && is_hls_manifest_media_url(&media.source) {
        // Keep live manifests in the HLS demuxer as their media window advances.
        Some("demuxer=+lavf,demuxer-lavf-format=hls".to_owned())
    } else if resume > 0. {
        Some(format!("start={resume}"))
    } else {
        None
    };
    match options {
        Some(options) if crate::player::loadfile_has_playlist_index() => {
            mpv.command("loadfile", &[&media.source, "replace", "-1", &options])
        }
        Some(options) => mpv.command("loadfile", &[&media.source, "replace", &options]),
        None => mpv.command("loadfile", &[&media.source, "replace"]),
    }
}

pub(super) fn same_local_path(left: &Path, right: &Path) -> bool {
    left == right || path_key(left) == path_key(right)
}

pub(super) fn load_sidecar_subtitles(mpv: &Mpv, media_path: &Path) -> Vec<i64> {
    let Some(media_path) = std::path::absolute(media_path).ok() else {
        return Vec::new();
    };
    let sidecars = discover_sidecar_subtitles(&media_path);
    if sidecars.is_empty() {
        return Vec::new();
    }

    let mut tracks = external_subtitles(mpv);
    // Keep IDs mpv assigned to existing tracks so remembered selections still resolve.
    let mut added = false;
    for path in &sidecars {
        if !tracks.contains_key(&path_key(path)) {
            added |= mpv
                .command("sub-add", &[&path.to_string_lossy(), "auto"])
                .is_ok();
        }
    }
    if added {
        tracks = external_subtitles(mpv);
    }

    sidecars
        .iter()
        .filter_map(|path| tracks.get(&path_key(path)).copied())
        .collect()
}

pub(super) fn has_subtitle_track(mpv: &Mpv, id: i64) -> bool {
    subtitle_tracks(mpv).any(|(_, track_id)| track_id == id)
}

pub(super) fn track_count(mpv: &Mpv) -> i64 {
    mpv.get_property::<i64>("track-list/count")
        .unwrap_or(0)
        .clamp(0, 1000)
}

fn subtitle_tracks(mpv: &Mpv) -> impl Iterator<Item = (String, i64)> + '_ {
    (0..track_count(mpv)).filter_map(|index| {
        let track = format!("track-list/{index}");
        let subtitle = mpv
            .get_property::<String>(&format!("{track}/type"))
            .is_ok_and(|kind| kind == "sub");
        if !subtitle {
            return None;
        }
        let id = mpv.get_property::<i64>(&format!("{track}/id")).ok()?;
        Some((track, id))
    })
}

fn external_subtitles(mpv: &Mpv) -> HashMap<String, i64> {
    subtitle_tracks(mpv)
        .filter_map(|(track, id)| {
            let path = mpv
                .get_property::<String>(&format!("{track}/external-filename"))
                .ok()?;
            Some((path_key(Path::new(&path)), id))
        })
        .collect()
}

fn path_key(path: &Path) -> String {
    std::path::absolute(path)
        .as_deref()
        .unwrap_or(path)
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use libmpv2::events::Event;

    use super::*;

    #[test]
    fn adds_missing_sidecars_in_order_without_duplicating_autoloaded_tracks() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "openplayer-sidecars-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let video = directory.join("节目.y4m");
        let frame = b"FRAME\n\x50\x50\x50\x50\x80\x80";
        let mut video_bytes = b"YUV4MPEG2 W2 H2 F1:1 Ip A1:1 C420jpeg\n".to_vec();
        video_bytes.extend(frame.repeat(4));
        fs::write(&video, video_bytes).unwrap();
        let names = [
            "节目.srt",
            "节目-commentary.srt",
            "节目.eng.srt",
            "节目_alt.srt",
        ];
        for name in names.into_iter().chain(["节目2.srt"]) {
            fs::write(
                directory.join(name),
                b"1\n00:00:00,000 --> 00:00:02,000\nHello\n",
            )
            .unwrap();
        }

        let mpv = Mpv::with_initializer(|init| {
            init.set_option("vo", "null")?;
            init.set_option("ao", "null")?;
            init.set_option("pause", true)?;
            Ok(())
        })
        .unwrap();
        loadfile(
            &mpv,
            &Media::local(video.to_string_lossy().into_owned()),
            0.,
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline, "headless fixture did not load");
            if matches!(mpv.wait_event(0.1), Some(Ok(Event::FileLoaded))) {
                break;
            }
        }

        let existing = external_subtitles(&mpv);
        assert!(
            !existing.is_empty(),
            "mpv should already have one of the exact matches"
        );
        let ordered = load_sidecar_subtitles(&mpv, &video);
        let tracks = external_subtitles(&mpv);
        let expected: Vec<_> = names
            .into_iter()
            .map(|name| tracks[&path_key(&directory.join(name))])
            .collect();
        assert_eq!(ordered, expected);
        assert_eq!(subtitle_tracks(&mpv).count(), names.len());
        for (path, id) in existing {
            assert_eq!(tracks[&path], id);
        }
        assert_eq!(load_sidecar_subtitles(&mpv, &video), ordered);
        assert_eq!(subtitle_tracks(&mpv).count(), names.len());

        drop(mpv);
        fs::remove_file(video).unwrap();
        for name in names.into_iter().chain(["节目2.srt"]) {
            fs::remove_file(directory.join(name)).unwrap();
        }
        fs::remove_dir(directory).unwrap();
    }
}

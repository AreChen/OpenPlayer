use std::{
    ffi::{CStr, CString},
    fs,
    path::{Component, Path, Prefix},
    ptr,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use libmpv2::Mpv;
use libmpv2_sys as sys;
use serde::Serialize;

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct MediaInfo {
    pub(super) format: Option<String>,
    pub(super) size_bytes: Option<u64>,
    pub(super) duration_seconds: Option<f64>,
    pub(super) bitrate_bps: Option<u64>,
    pub(super) video: Option<VideoInfo>,
    pub(super) audio: Option<AudioInfo>,
    pub(super) tracks: Vec<MediaTrackInfo>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct VideoInfo {
    pub(super) codec: Option<String>,
    pub(super) decoder: Option<String>,
    pub(super) format: Option<String>,
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) fps: Option<f64>,
    pub(super) bitrate_bps: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct AudioInfo {
    pub(super) codec: Option<String>,
    pub(super) decoder: Option<String>,
    pub(super) format: Option<String>,
    pub(super) channels: Option<u32>,
    pub(super) sample_rate_hz: Option<u32>,
    pub(super) bitrate_bps: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct MediaTrackInfo {
    pub(super) id: i64,
    pub(super) kind: String,
    pub(super) title: Option<String>,
    pub(super) language: Option<String>,
    pub(super) codec: Option<String>,
    pub(super) selected: bool,
}

pub(super) fn snapshot(mpv: &Mpv) -> MediaInfo {
    let size_bytes = positive_integer(mpv, "file-size");
    let duration_seconds = positive_double(mpv, "duration");
    let (video, audio, tracks) = read_tracks(mpv);
    let bitrate_bps = average_bitrate(size_bytes, duration_seconds).or_else(|| {
        let video_bitrate = video.as_ref().and_then(|info| info.bitrate_bps);
        let audio_bitrate = audio.as_ref().and_then(|info| info.bitrate_bps);
        (video_bitrate.is_some() || audio_bitrate.is_some())
            .then(|| video_bitrate.unwrap_or_default() + audio_bitrate.unwrap_or_default())
    });

    MediaInfo {
        format: string(mpv, "file-format"),
        size_bytes,
        duration_seconds,
        bitrate_bps,
        video,
        audio,
        tracks,
    }
}

fn read_tracks(mpv: &Mpv) -> (Option<VideoInfo>, Option<AudioInfo>, Vec<MediaTrackInfo>) {
    let mut video = None;
    let mut audio = None;
    let mut tracks = Vec::new();
    let count = mpv
        .get_property::<i64>("track-list/count")
        .unwrap_or_default();

    for index in 0..count {
        let props = TrackProperties {
            mpv,
            prefix: format!("track-list/{index}"),
        };
        let Some(id) = props.get("id") else {
            continue;
        };
        let Some(kind) = props.string("type") else {
            continue;
        };
        let codec = props.string("codec");
        let selected = props.get("selected").unwrap_or_default();

        match kind.as_str() {
            "video"
                if (video.is_none() || selected)
                    && !props.get::<bool>("albumart").unwrap_or_default() =>
            {
                video = Some(read_video(&props, codec.clone(), selected));
            }
            "audio" if audio.is_none() || selected => {
                audio = Some(read_audio(&props, codec.clone(), selected));
            }
            _ => {}
        }

        tracks.push(MediaTrackInfo {
            id,
            kind,
            title: props.string("title"),
            language: props.string("lang"),
            codec,
            selected,
        });
    }

    (video, audio, tracks)
}

fn read_video(props: &TrackProperties<'_>, codec: Option<String>, selected: bool) -> VideoInfo {
    let live = selected.then_some(props.mpv);
    VideoInfo {
        codec,
        decoder: props.string("decoder"),
        format: live
            .and_then(|mpv| string(mpv, "video-params/pixelformat"))
            .or_else(|| props.string("format-name")),
        width: live
            .and_then(|mpv| positive_u32(mpv.get_property("width").ok()))
            .or_else(|| positive_u32(props.get("demux-w"))),
        height: live
            .and_then(|mpv| positive_u32(mpv.get_property("height").ok()))
            .or_else(|| positive_u32(props.get("demux-h"))),
        fps: positive(props.get("demux-fps"))
            .or_else(|| live.and_then(|mpv| positive_double(mpv, "container-fps"))),
        bitrate_bps: positive_u64(props.get("demux-bitrate"))
            .or_else(|| live.and_then(|mpv| positive_integer(mpv, "video-bitrate"))),
    }
}

fn read_audio(props: &TrackProperties<'_>, codec: Option<String>, selected: bool) -> AudioInfo {
    let live = selected.then_some(props.mpv);
    AudioInfo {
        codec,
        decoder: props.string("decoder"),
        format: live
            .and_then(|mpv| string(mpv, "audio-params/format"))
            .or_else(|| props.string("format-name")),
        channels: live
            .and_then(|mpv| positive_u32(mpv.get_property("audio-params/channel-count").ok()))
            .or_else(|| positive_u32(props.get("demux-channel-count"))),
        sample_rate_hz: live
            .and_then(|mpv| positive_u32(mpv.get_property("audio-params/samplerate").ok()))
            .or_else(|| positive_u32(props.get("demux-samplerate"))),
        bitrate_bps: positive_u64(props.get("demux-bitrate"))
            .or_else(|| live.and_then(|mpv| positive_integer(mpv, "audio-bitrate"))),
    }
}

struct TrackProperties<'a> {
    mpv: &'a Mpv,
    prefix: String,
}

impl TrackProperties<'_> {
    fn get<T: libmpv2::GetData>(&self, name: &str) -> Option<T> {
        self.mpv
            .get_property(&format!("{}/{name}", self.prefix))
            .ok()
    }

    fn string(&self, name: &str) -> Option<String> {
        self.get::<String>(name)
            .filter(|value| !value.trim().is_empty())
    }
}

fn string(mpv: &Mpv, name: &str) -> Option<String> {
    mpv.get_property::<String>(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn positive_integer(mpv: &Mpv, name: &str) -> Option<u64> {
    positive_u64(mpv.get_property(name).ok())
}

fn positive_double(mpv: &Mpv, name: &str) -> Option<f64> {
    positive(mpv.get_property(name).ok())
}

fn positive_u32(value: Option<i64>) -> Option<u32> {
    positive_u64(value).and_then(|value| u32::try_from(value).ok())
}

fn positive_u64(value: Option<i64>) -> Option<u64> {
    value.filter(|value| *value > 0).map(|value| value as u64)
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn average_bitrate(size: Option<u64>, duration: Option<f64>) -> Option<u64> {
    let bitrate = (size? as f64 * 8.0 / duration?).round();
    (bitrate.is_finite() && bitrate > 0.0 && bitrate < u64::MAX as f64).then_some(bitrate as u64)
}

pub(super) fn probe_local(path: &Path) -> Result<MediaInfo> {
    let path = std::path::absolute(path)?;
    ensure!(
        is_filesystem_path(&path),
        "Only filesystem paths can be inspected"
    );
    let metadata = fs::metadata(&path).context("Media file is unavailable")?;
    ensure!(metadata.is_file(), "The selected path is not a file");
    let path = CString::new(path.to_str().context("Media path is not valid UTF-8")?)?;

    let mpv = Mpv::with_initializer(|initializer| {
        for (name, value) in [
            ("config", "no"),
            ("load-scripts", "no"),
            ("ytdl", "no"),
            ("autoload-files", "no"),
            ("access-references", "no"),
            ("stream-lavf-o", "protocol_whitelist=file"),
            ("demuxer-lavf-o", "protocol_whitelist=file"),
            ("cache", "no"),
            ("demuxer-max-bytes", "4MiB"),
            ("pause", "yes"),
            ("vo", "null"),
            ("ao", "null"),
            ("vid", "no"),
            ("aid", "no"),
            ("sid", "no"),
        ] {
            initializer.set_option(name, value)?;
        }
        Ok(())
    })
    .context("Could not initialize media inspection")?;

    let mut info = inspect_headers(&mpv, &path)?;
    ensure!(
        info.tracks
            .iter()
            .any(|track| matches!(track.kind.as_str(), "audio" | "video")),
        "No audio or video streams were found"
    );
    info.size_bytes = Some(metadata.len());
    info.bitrate_bps = average_bitrate(info.size_bytes, info.duration_seconds).or(info.bitrate_bps);
    Ok(info)
}

fn inspect_headers(mpv: &Mpv, path: &CStr) -> Result<MediaInfo> {
    let handle = mpv.ctx.as_ptr();
    let mut args = [c"loadfile".as_ptr(), path.as_ptr(), ptr::null()];
    // At on_preloaded, the demuxer has opened the file and decoders have not started.
    unsafe {
        check_mpv(sys::mpv_hook_add(handle, 1, c"on_preloaded".as_ptr(), 0))?;
        check_mpv(sys::mpv_command_async(handle, 2, args.as_mut_ptr()))?;
    }
    let deadline = Instant::now() + PROBE_TIMEOUT;

    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        // The handle is confined to this thread and event data remains valid
        // until the next mpv_wait_event call.
        let event = unsafe { &*sys::mpv_wait_event(handle, remaining.as_secs_f64()) };
        match event.event_id {
            sys::mpv_event_id_MPV_EVENT_HOOK => {
                let hook = unsafe { &*event.data.cast::<sys::mpv_event_hook>() };
                let info = snapshot(mpv);
                check_mpv(unsafe { sys::mpv_hook_continue(handle, hook.id) })?;
                return Ok(info);
            }
            sys::mpv_event_id_MPV_EVENT_COMMAND_REPLY => check_mpv(event.error)?,
            sys::mpv_event_id_MPV_EVENT_END_FILE => {
                let end = unsafe { &*event.data.cast::<sys::mpv_event_end_file>() };
                check_mpv(end.error)?;
                bail!("No media streams were found");
            }
            sys::mpv_event_id_MPV_EVENT_SHUTDOWN => bail!("Media inspection stopped"),
            _ => {}
        }
    }
    bail!("Media inspection timed out")
}

fn check_mpv(status: i32) -> Result<()> {
    if status >= 0 {
        return Ok(());
    }
    // libmpv returns a static string for each error code.
    let message = unsafe { CStr::from_ptr(sys::mpv_error_string(status)) }.to_string_lossy();
    Err(anyhow!("{message}"))
}

fn is_filesystem_path(path: &Path) -> bool {
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return false;
    };
    match prefix.kind() {
        Prefix::Disk(_) | Prefix::VerbatimDisk(_) => path.is_absolute(),
        Prefix::UNC(_, share) | Prefix::VerbatimUNC(_, share) => {
            share.to_str().is_some_and(|share| {
                let share = share.trim_end_matches(['.', ' ']);
                !share.is_empty()
                    && !["pipe", "mailslot", "ipc$"]
                        .iter()
                        .any(|name| share.eq_ignore_ascii_case(name))
            })
        }
        _ => false,
    }
}

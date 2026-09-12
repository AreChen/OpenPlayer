//! Host-generated single-owner filter. Public callers resolve a declared runtime
//! and an authorized native session; they cannot supply scripts or filter graphs.
use super::*;
pub(in crate::mpv_embed) mod status;
use std::{
    io::Write,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

static CLAIMED: AtomicBool = AtomicBool::new(false);
static SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct OwnedFilter {
    app: AppHandle,
    script: PathBuf,
    label: String,
    script_created: bool,
    normalize: bool,
    frame_rate_limit: Option<f64>,
}

impl OwnedFilter {
    pub(crate) fn prepare(
        app: AppHandle,
        directory: &Path,
        endpoint: &Value,
    ) -> Result<Self, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        if CLAIMED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("another native video attachment owns the player".into());
        }
        let label = format!(
            "native-owned-{}-{}-{}",
            std::process::id(),
            timestamp,
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let mut filter = Self {
            app,
            script: directory.join(format!("{label}.vpy")),
            label,
            script_created: false,
            normalize: false,
            frame_rate_limit: None,
        };
        let encoded = serde_json::to_string(&endpoint.to_string()).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filter.script)
            .map_err(|e| e.to_string())?;
        filter.script_created = true;
        write!(file, "import json\nfrom host_vapoursynth import create_filter\ncreate_filter(video_in, json.loads({encoded})).set_output()\n")
            .map_err(|e| e.to_string())?;
        Ok(filter)
    }

    pub(crate) fn normalize_to_sdr(&mut self, enabled: bool) {
        self.normalize = enabled;
    }

    fn conversion_label(&self) -> String {
        self.label.replacen("native-owned-", "native-input-", 1)
    }

    pub(crate) fn limit_frame_rate(&mut self, rate: Option<f64>) {
        self.frame_rate_limit = rate;
    }

    pub(crate) fn install(&self) -> Result<(), String> {
        let script = self.script.to_str().ok_or("non-UTF8 adapter path")?;
        let filter = format!(
            "@{}:vapoursynth=file=%{}%{}:concurrent-frames=1",
            self.label,
            script.len(),
            script
        );
        let state = self.app.state::<MpvEmbedState>();
        with_player(state.inner(), |player| {
            let mut chain = Vec::new();
            if self.normalize || self.frame_rate_limit.is_some() {
                // Force mpv's hardware download before libavfilter negotiation.
                // Preserve 10-bit DV samples; libplacebo performs the color conversion.
                let format = if self.normalize {
                    "yuv420p10"
                } else {
                    "yuv420p"
                };
                let download = format!("@{}-download:format=fmt={format}", self.conversion_label());
                chain.push(download);
            }
            if let Some(limit) = self.frame_rate_limit {
                let source = player
                    .mpv
                    .get_property::<f64>("container-fps")
                    .unwrap_or(limit);
                let rate = if source.is_finite() && source > 0.0 {
                    source.min(limit)
                } else {
                    limit
                };
                // Drop before normalization/NR, preserving media time and audio speed.
                chain.push(format!(
                    "@{}-rate:lavfi=[fps=fps={rate}]",
                    self.conversion_label()
                ));
            }
            if self.normalize {
                // Interpret Dolby Vision RPU before RGB processing, then strip it.
                // Fixed host-owned graph: no arbitrary filter text crosses the SDK.
                let conversion = super::native_video_color::sdr_filter(&self.conversion_label());
                chain.push(conversion);
            }
            chain.push(filter);
            player
                .mpv
                .command("vf", &["add", &chain.join(",")])
                .map_err(|e| e.to_string())
        })
    }

    pub(crate) fn remove(&self) -> Result<(), String> {
        let state = self.app.state::<MpvEmbedState>();
        let player = state.player.lock().map_err(|_| "mpv state unavailable")?;
        // No player means its filter was already destroyed on stop/replacement.
        if let Some(player) = player.as_ref() {
            let filters = status::filters(&player.mpv)?;
            let labels = [
                &self.label,
                &self.conversion_label(),
                &format!("{}-download", self.conversion_label()),
                &format!("{}-rate", self.conversion_label()),
            ]
            .into_iter()
            .filter(|owned| filters.iter().any(|(label, _)| label == *owned))
            .map(|owned| format!("@{owned}"))
            .collect::<Vec<_>>();
            if !labels.is_empty() {
                player
                    .mpv
                    .command("vf", &["remove", &labels.join(",")])
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

pub(crate) fn enabled(app: &AppHandle) -> Result<bool, String> {
    let state = app.state::<MpvEmbedState>();
    let player = state.player.lock().map_err(|_| "mpv state unavailable")?;
    let Some(player) = player.as_ref() else {
        return Ok(false);
    };
    Ok(enabled_filters(&status::filters(&player.mpv)?))
}

pub(crate) fn refresh_paused(app: &AppHandle) -> Result<bool, String> {
    with_player(app.state::<MpvEmbedState>().inner(), |player| {
        if !player.mpv.get_property::<bool>("pause").unwrap_or(false) {
            return Ok(false);
        }
        if !player.mpv.get_property::<bool>("seekable").unwrap_or(false) {
            return Err("paused frame refresh requires seekable media".into());
        }
        // A zero-distance exact seek invalidates cached filter frames without
        // unpausing or advancing by one frame. The existing owner reuses its worker.
        player
            .mpv
            .command("seek", &["0", "relative+exact"])
            .map_err(|e| e.to_string())?;
        Ok(true)
    })
}

fn enabled_filters(filters: &[(String, bool)]) -> bool {
    filters
        .iter()
        .any(|(label, enabled)| label.starts_with("native-owned-") && *enabled)
        && filters
            .iter()
            .all(|(label, enabled)| !label.starts_with("native-input-") || *enabled)
}

pub(crate) fn validate_media(app: &AppHandle, normalize: bool) -> Result<bool, String> {
    with_player(app.state::<MpvEmbedState>().inner(), |player| {
        let mpv = &player.mpv;
        let width = mpv.get_property::<i64>("video-params/w").unwrap_or(0);
        let height = mpv.get_property::<i64>("video-params/h").unwrap_or(0);
        let hardware = mpv
            .get_property::<String>("video-params/hw-pixelformat")
            .unwrap_or_default();
        let format = if hardware.is_empty() {
            mpv.get_property::<String>("video-params/pixelformat")
                .unwrap_or_default()
        } else {
            hardware
        };
        let matrix = mpv
            .get_property::<String>("video-params/colormatrix")
            .unwrap_or_default();
        let gamma = mpv
            .get_property::<String>("video-params/gamma")
            .unwrap_or_default();
        let levels = mpv
            .get_property::<String>("video-params/colorlevels")
            .unwrap_or_default();
        if !(16..=3840).contains(&width) || !(16..=2160).contains(&height) {
            return Err(format!(
                "native video dimensions must be 16..3840 x 16..2160; current: {width}x{height}"
            ));
        }
        let needs_conversion =
            !matches!(format.as_str(), "yuv420p" | "yuv422p" | "yuv444p" | "nv12")
                || matrix != "bt.709"
                || levels != "limited"
                || !matches!(gamma.as_str(), "bt.1886" | "bt.709" | "srgb");
        if !normalize && needs_conversion {
            return Err(format!(
                "native video requires 8-bit limited-range BT.709 SDR or inputConversion=sdr-bt709; current: {width}x{height} {format} {matrix} {gamma} {levels}"
            ));
        }
        Ok(needs_conversion)
    })
}

impl Drop for OwnedFilter {
    fn drop(&mut self) {
        if self.script_created {
            let _ = std::fs::remove_file(&self.script);
        }
        CLAIMED.store(false, Ordering::Release);
    }
}

#[cfg(feature = "window-smoke")]
pub(crate) fn filters(app: &AppHandle) -> Result<String, String> {
    with_player(app.state::<MpvEmbedState>().inner(), |p| {
        p.mpv
            .get_property::<String>("vf")
            .map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::enabled_filters;
    #[test]
    fn failed_conversion_is_not_an_enabled_attachment() {
        assert!(enabled_filters(&[("native-owned-a".into(), true)]));
        assert!(!enabled_filters(&[("native-input-a".into(), true)]));
        assert!(!enabled_filters(&[
            ("native-owned-a".into(), true),
            ("native-input-a".into(), false)
        ]));
        assert!(!enabled_filters(&[
            ("native-owned-a".into(), false),
            ("native-input-a".into(), true)
        ]));
    }
}

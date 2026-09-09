//! Host-generated single-owner filter. Public callers resolve a declared runtime
//! and an authorized native session; they cannot supply scripts or filter graphs.
use super::*;
mod status;
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
            player
                .mpv
                .command("vf", &["add", &filter])
                .map_err(|e| e.to_string())
        })
    }

    pub(crate) fn remove(&self) -> Result<(), String> {
        let state = self.app.state::<MpvEmbedState>();
        let player = state.player.lock().map_err(|_| "mpv state unavailable")?;
        // No player means its filter was already destroyed on stop/replacement.
        if let Some(player) = player.as_ref() {
            if !status::filters(&player.mpv)?
                .iter()
                .any(|(label, _)| label == &self.label)
            {
                return Ok(());
            }
            player
                .mpv
                .command("vf", &["remove", &format!("@{}", self.label)])
                .map_err(|e| e.to_string())?;
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
    Ok(status::filters(&player.mpv)?
        .iter()
        .any(|(label, enabled)| label.starts_with("native-owned-") && *enabled))
}

pub(crate) fn validate_media(app: &AppHandle) -> Result<(), String> {
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
        if !(16..=1920).contains(&width)
            || !(16..=1080).contains(&height)
            || !matches!(format.as_str(), "yuv420p" | "yuv422p" | "yuv444p" | "nv12")
            || matrix != "bt.709"
            || levels != "limited"
            || !matches!(gamma.as_str(), "bt.1886" | "bt.709" | "srgb")
        {
            return Err(format!(
                "native video requires 8-bit limited-range BT.709 SDR up to 1080p; current: {width}x{height} {format} {matrix} {gamma} {levels}"
            ));
        }
        Ok(())
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

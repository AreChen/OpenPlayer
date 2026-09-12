//! Generic out-of-process presentation. The vendor SDK never enters this process.
mod cadence;
mod render;
mod transaction;
use super::{MpvEmbedPlayer, MpvEmbedState, with_player};
use openplayer_native_sdk::presentation::windows::{Control, Producer};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tauri::{AppHandle, Manager};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) struct Prepared {
    app: AppHandle,
    id: u64,
    core: usize,
    producer: Mutex<Option<Producer>>,
    pub(crate) control: Control,
    pub(crate) parameters: Value,
    width: u32,
    height: u32,
    fps: f64,
    rate: Option<f64>,
}

pub(crate) struct Presentation {
    id: u64,
    worker: render::Worker,
    saved: transaction::SavedOutput,
    restored: bool,
    control_client: libmpv2::Mpv,
}

impl Prepared {
    pub(crate) fn new(app: AppHandle, options: Value) -> Result<Arc<Self>, String> {
        let mut options = options
            .as_object()
            .cloned()
            .ok_or("presentation options must be an object")?;
        for reserved in ["endpoint", "parentWindow", "sourceFps"] {
            if options.contains_key(reserved) {
                return Err(format!("{reserved} is host-owned"));
            }
        }
        let width = dimension(options.remove("width"), 1920, 3840)?;
        let height = dimension(options.remove("height"), 1080, 2160)?;
        let rate = options
            .remove("frameRateLimit")
            .map(|value| {
                value
                    .as_f64()
                    .filter(|n| n.is_finite() && (1.0..=120.0).contains(n))
                    .ok_or_else(|| "frameRateLimit must be a number from 1 to 120".to_string())
            })
            .transpose()?;
        let (core, parent, fps) = with_player(app.state::<MpvEmbedState>().inner(), |player| {
            if player.presentation.is_some() {
                return Err("another native presenter owns the video output".into());
            }
            validate_media(&player.mpv)?;
            let source = player
                .mpv
                .get_property::<f64>("estimated-vf-fps")
                .ok()
                .filter(|fps| fps.is_finite() && *fps > 0.0)
                .or_else(|| {
                    player
                        .mpv
                        .get_property::<f64>("container-fps")
                        .ok()
                        .filter(|fps| fps.is_finite() && *fps > 0.0)
                })
                .ok_or("native presentation requires a known source cadence")?;
            Ok((
                player.mpv.ctx.as_ptr() as usize,
                player.host.wid(),
                source.min(rate.unwrap_or(120.0)),
            ))
        })?;
        let producer = Producer::create(width * height * 4).map_err(|e| e.to_string())?;
        let (initial_width, initial_height) =
            render::viewport_size(parent as isize, width, height)?;
        options.insert("endpoint".into(), json!(producer.endpoint()));
        options.insert(
            "parentWindow".into(),
            json!(format!("{:016x}", parent as u64)),
        );
        options.insert("width".into(), json!(initial_width));
        options.insert("height".into(), json!(initial_height));
        options.insert("sourceFps".into(), json!(fps));
        Ok(Arc::new(Self {
            app,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            core,
            control: producer.control(),
            producer: Mutex::new(Some(producer)),
            parameters: Value::Object(options),
            width,
            height,
            fps,
            rate,
        }))
    }
    pub(crate) fn install(&self) -> Result<(), String> {
        let result = with_player(self.app.state::<MpvEmbedState>().inner(), |player| {
            if player.mpv.ctx.as_ptr() as usize != self.core {
                return Err("media changed before presentation attachment".into());
            }
            if player.presentation.is_some() {
                return Err("another native presenter owns the video output".into());
            }
            if self.control.is_closed() {
                return Err("native presenter closed during initialization".into());
            }
            let producer = self
                .producer
                .lock()
                .map_err(|_| "presentation producer unavailable")?
                .take()
                .ok_or("presentation already installed")?;
            let saved = transaction::SavedOutput::read(&player.mpv)?;
            let control_client = player.mpv.create_client(None).map_err(|e| e.to_string())?;
            let worker = render::Worker::start(
                &player.mpv,
                producer,
                render::Output {
                    width: self.width,
                    height: self.height,
                    fps: self.fps,
                    window: player.host.wid() as isize,
                    limit: self.rate,
                },
            )?;
            player.presentation = Some(Presentation {
                id: self.id,
                worker,
                saved,
                restored: false,
                control_client,
            });
            let presentation = player.presentation.as_mut().unwrap();
            presentation
                .worker
                .shared
                .active
                .store(true, Ordering::Release);
            transaction::switch(&player.mpv, "libmpv", "no", &presentation.worker.shared)?;
            Ok(())
        });
        schedule_resize(&self.app);
        result
    }
    pub(crate) fn remove(&self) -> Result<(), String> {
        self.control.close();
        let state = self.app.state::<MpvEmbedState>();
        let mut guard = state.player.lock().map_err(|_| "mpv state unavailable")?;
        if let Some(player) = guard.as_mut() {
            let Some(presentation) = player.presentation.as_mut().filter(|p| p.id == self.id)
            else {
                return Ok(());
            };
            if crate::plugin_native::video::media_teardown_active() {
                presentation
                    .worker
                    .shared
                    .active
                    .store(false, Ordering::Release);
                player.mpv.command("stop", &[]).map_err(|e| e.to_string())?;
                presentation.restored = true;
                presentation.worker.stop()?;
            } else {
                presentation.restore(&player.mpv)?;
            }
            player.presentation = None;
        }
        drop(guard);
        schedule_resize(&self.app);
        Ok(())
    }
}

fn schedule_resize(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = handle.state::<MpvEmbedState>().resize_video_host();
    });
}

impl Presentation {
    fn restore(&mut self, mpv: &libmpv2::Mpv) -> Result<(), String> {
        if !self.restored {
            self.worker.shared.active.store(false, Ordering::Release);
            self.worker.shared.invalidate();
            transaction::switch(mpv, &self.saved.vo, &self.saved.hwdec, &self.worker.shared)?;
            self.restored = true;
        }
        self.worker.stop()
    }
}
impl Drop for Presentation {
    fn drop(&mut self) {
        if !self.restored {
            // Last-resort player teardown: keep pumping the render contract until
            // stop completes, then the Worker drops its render context and client.
            self.worker.shared.active.store(false, Ordering::Release);
            self.worker.shared.control.close();
            let _ = self.control_client.command("stop", &[]);
        }
    }
}

pub(crate) fn active(app: &AppHandle) -> Result<bool, String> {
    let state = app.state::<MpvEmbedState>();
    let guard = state.player.lock().map_err(|_| "mpv state unavailable")?;
    Ok(guard
        .as_ref()
        .and_then(|p| p.presentation.as_ref())
        .is_some_and(|p| {
            p.worker.shared.active.load(Ordering::Acquire) && !p.worker.shared.control.is_closed()
        }))
}
pub(in crate::mpv_embed) fn invalidate(player: &MpvEmbedPlayer, paused: Option<bool>) {
    if let Some(presentation) = &player.presentation {
        if let Some(paused) = paused {
            presentation
                .worker
                .shared
                .paused
                .store(paused, Ordering::Release);
        }
        presentation.worker.shared.invalidate();
    }
}
fn dimension(value: Option<Value>, default: u32, max: u32) -> Result<u32, String> {
    value
        .map(|value| {
            value
                .as_u64()
                .filter(|n| (128..=max as u64).contains(n))
                .map(|n| n as u32)
                .ok_or_else(|| {
                    format!("presentation dimension must be an integer from 128 to {max}")
                })
        })
        .unwrap_or(Ok(default))
}
fn validate_media(mpv: &libmpv2::Mpv) -> Result<(), String> {
    if !mpv.get_property::<bool>("seekable").unwrap_or(false) {
        return Err("native presentation currently requires seekable media".into());
    }
    let gamma = mpv
        .get_property::<String>("video-out-params/gamma")
        .unwrap_or_default();
    let matrix = mpv
        .get_property::<String>("video-out-params/colormatrix")
        .unwrap_or_default();
    if !matches!(gamma.as_str(), "bt.1886" | "bt.709" | "srgb") || matrix != "bt.709" {
        return Err(format!(
            "native presentation requires SDR BT.709 filter output; current: {matrix} {gamma}. Enable an SDR conversion filter before presentation."
        ));
    }
    Ok(())
}

#[cfg(feature = "window-smoke")]
pub(crate) fn diagnostics(app: &AppHandle) -> Result<Value, String> {
    with_player(app.state::<MpvEmbedState>().inner(), |player| {
        let presentation = player.presentation.as_ref();
        Ok(json!({ "core": player.mpv.ctx.as_ptr() as usize,
            "vo": player.mpv.get_property::<String>("vo").ok(),
            "currentVo": player.mpv.get_property::<String>("current-vo").ok(),
            "hwdec": player.mpv.get_property::<String>("hwdec").ok(),
            "presentationActive": presentation.is_some_and(|p| p.worker.shared.active.load(Ordering::Acquire)),
            "lastHash": presentation.map(|p| p.worker.shared.last_hash.load(Ordering::Acquire)),
            "sent": presentation.map(|p| p.worker.shared.sent.load(Ordering::Acquire)),
            "dropped": presentation.map(|p| p.worker.shared.dropped.load(Ordering::Acquire)) }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_reject_fractional_and_out_of_range_values() {
        for value in [
            json!(0),
            json!(-1),
            json!(127),
            json!(3841),
            json!(128.5),
            json!(null),
            json!("1920"),
        ] {
            assert!(dimension(Some(value), 1920, 3840).is_err());
        }
        assert_eq!(dimension(None, 1920, 3840).unwrap(), 1920);
    }
}

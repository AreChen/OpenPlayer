use super::render::Shared;
use libmpv2::{Mpv, events::Event};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

pub(super) struct SavedOutput {
    pub vo: String,
    pub hwdec: String,
}
impl SavedOutput {
    pub fn read(mpv: &Mpv) -> Result<Self, String> {
        Ok(Self {
            vo: mpv.get_property("vo").map_err(|e| e.to_string())?,
            hwdec: mpv.get_property("hwdec").map_err(|e| e.to_string())?,
        })
    }
}

pub(super) fn switch(mpv: &Mpv, vo: &str, hwdec: &str, shared: &Shared) -> Result<(), String> {
    let paused = mpv
        .get_property::<bool>("pause")
        .map_err(|e| e.to_string())?;
    let position = mpv
        .get_property::<f64>("time-pos")
        .map_err(|e| e.to_string())?;
    let track = mpv
        .get_property::<String>("vid")
        .map_err(|e| e.to_string())?;
    let before = SavedOutput::read(mpv)?;
    shared.paused.store(true, Ordering::Release);
    shared.invalidate();
    let result = apply(mpv, vo, hwdec, &track, position);
    if let Err(error) = result {
        let rollback = apply(mpv, &before.vo, &before.hwdec, &track, position);
        let resume = mpv.set_property("pause", paused).map_err(|e| e.to_string());
        shared.paused.store(paused, Ordering::Release);
        shared.invalidate();
        return Err(format!(
            "native output switch failed: {error}; rollback: {rollback:?}; pause restoration: {resume:?}"
        ));
    }
    mpv.set_property("pause", paused)
        .map_err(|e| e.to_string())?;
    shared.paused.store(paused, Ordering::Release);
    shared.invalidate();
    Ok(())
}
fn apply(mpv: &Mpv, vo: &str, hwdec: &str, track: &str, position: f64) -> Result<(), String> {
    // A plain VO change leaves the pinned AV1 decoder without a keyframe. Reset
    // only the selected video track, retaining the same core/audio and media time.
    mpv.set_property("pause", true).map_err(|e| e.to_string())?;
    mpv.set_property("vid", "no").map_err(|e| e.to_string())?;
    mpv.set_property("hwdec", hwdec)
        .map_err(|e| e.to_string())?;
    mpv.set_property("vo", vo).map_err(|e| e.to_string())?;
    let events = mpv.create_client(None).map_err(|e| e.to_string())?;
    mpv.set_property("vid", track).map_err(|e| e.to_string())?;
    mpv.command("seek", &[&position.to_string(), "absolute+exact"])
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if Instant::now() >= deadline {
            return Err("native output seek did not settle".into());
        }
        // 'seeking' can still be false before the asynchronous seek begins.
        // Do not release the render context or accept a new presenter until mpv
        // has produced the first frame on the replacement output.
        if matches!(events.wait_event(0.05), Some(Ok(Event::PlaybackRestart)))
            && !mpv.get_property::<bool>("seeking").unwrap_or(true)
            && mpv.get_property::<i64>("video-out-params/w").unwrap_or(0) > 0
        {
            return Ok(());
        }
    }
}

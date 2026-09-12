use super::render::Shared;
use libmpv2::{Mpv, events::Event};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct SavedOutput {
    pub vo: String,
    pub hwdec: String,
    pub cuda_device: Option<String>,
}
impl SavedOutput {
    pub fn read(mpv: &Mpv) -> Result<Self, String> {
        Ok(Self {
            vo: mpv.get_property("vo").map_err(|e| e.to_string())?,
            hwdec: mpv.get_property("hwdec").map_err(|e| e.to_string())?,
            cuda_device: mpv.get_property("cuda-decode-device").ok(),
        })
    }
}

pub(super) fn switch(mpv: &Mpv, output: &SavedOutput, shared: &Shared) -> Result<(), String> {
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
    let result = apply(mpv, output, &track, position);
    if let Err(error) = result {
        let rollback = apply(mpv, &before, &track, position);
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
pub(super) fn set_output(mpv: &Mpv, output: &SavedOutput) -> Result<(), String> {
    if let Some(device) = &output.cuda_device {
        mpv.set_property("cuda-decode-device", device.as_str())
            .map_err(|e| e.to_string())?;
    }
    mpv.set_property("hwdec", output.hwdec.as_str())
        .map_err(|e| e.to_string())?;
    mpv.set_property("vo", output.vo.as_str())
        .map_err(|e| e.to_string())
}

fn apply(mpv: &Mpv, output: &SavedOutput, track: &str, position: f64) -> Result<(), String> {
    // A plain VO change leaves the pinned AV1 decoder without a keyframe. Reset
    // only the selected video track, retaining the same core/audio and media time.
    mpv.set_property("pause", true).map_err(|e| e.to_string())?;
    mpv.set_property("vid", "no").map_err(|e| e.to_string())?;
    set_output(mpv, output)?;
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

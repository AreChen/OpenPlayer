use super::{Session, attachment::Attachment};
use crate::mpv_embed::native_presentation::Prepared;
use serde_json::Value;
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

pub(super) fn attach(
    app: &AppHandle,
    session: &Arc<Session>,
    slot: &mut Attachment,
    options: Value,
) -> Result<(), String> {
    let prepared = Prepared::new(app.clone(), options)?;
    let opened = tauri::async_runtime::block_on(session.request(
        "frames.open",
        prepared.parameters.clone(),
        5000,
    ))?;
    if opened["phase"] != "starting" && opened["phase"] != "ready" {
        return Err("native presenter did not enter initialization".into());
    }
    #[cfg(feature = "window-smoke")]
    if std::env::var_os("OPENPLAYER_SMOKE_CLOSE_DURING_PRESENTATION_INIT").is_some() {
        println!("TRACE: requesting close while presenter initialization owns the media guard");
        crate::window::request_native_close(app);
        thread::sleep(Duration::from_millis(100));
    }
    wait_phase(session, "ready", 30)?;
    let cleanup = prepared.clone();
    slot.mount(|| prepared.install(), Box::new(move || cleanup.remove()))?;
    let control = prepared.control.clone();
    // A host render failure closes the transport even if the control process is
    // alive. Reuse the native supervisor to kill it and restore the original VO.
    slot.watch_health(Box::new(move || !control.is_closed()));
    Ok(())
}

pub(super) fn wait_phase(session: &Session, phase: &str, seconds: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        let status =
            tauri::async_runtime::block_on(session.request("frames.status", Value::Null, 5000))?;
        if status["phase"] == phase {
            return Ok(());
        }
        if status["phase"] == "failed" || (phase == "ready" && status["phase"] == "closed") {
            return Err(format!(
                "native presenter stopped: {}",
                status["error"]
                    .as_str()
                    .unwrap_or("closed during initialization")
            ));
        }
        if Instant::now() >= deadline {
            return Err(format!("native presenter did not become {phase}"));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

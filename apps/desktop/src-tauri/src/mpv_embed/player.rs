use super::*;

pub(super) fn with_player<T>(
    state: &MpvEmbedState,
    callback: impl FnOnce(&mut MpvEmbedPlayer) -> Result<T, String>,
) -> Result<T, String> {
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    let player = player
        .as_mut()
        .ok_or_else(|| "mpv has no loaded media".to_string())?;

    callback(player)
}

pub(super) async fn run_mpv_command<T>(
    app: AppHandle,
    callback: impl FnOnce(&MpvEmbedState) -> Result<T, String> + Send + 'static,
) -> Result<T, String>
where
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvEmbedState>();
        callback(state.inner())
    })
    .await
    .map_err(|error| format!("mpv command task failed: {error}"))?
}

pub(super) fn frame_step(state: &MpvEmbedState, command: &str) -> Result<MpvEmbedSnapshot, String> {
    with_player(state, |player| {
        player
            .mpv
            .command(command, &[])
            .map_err(|error| format!("mpv {command} failed: {error}"))?;
        player.force_paused_until = Some(Instant::now() + FRAME_STEP_PAUSE_GUARD);
        settle_frame_step_pause(&player.mpv)?;
        Ok(player.snapshot(0, "paused"))
    })
}

pub(super) fn settle_frame_step_pause(mpv: &libmpv2::Mpv) -> Result<(), String> {
    thread::sleep(FRAME_STEP_SETTLE_INTERVAL);
    let deadline = Instant::now() + FRAME_STEP_SETTLE_TIMEOUT;
    while Instant::now() < deadline {
        if mpv.get_property::<bool>("pause").unwrap_or(false) {
            return Ok(());
        }
        thread::sleep(FRAME_STEP_SETTLE_INTERVAL);
    }

    mpv.set_property("pause", true)
        .map_err(|error| format!("mpv frame-step pause settle failed: {error}"))
}

impl MpvEmbedState {
    #[allow(dead_code)]
    pub fn resize_video_host(&self) -> Result<(), String> {
        let player = self
            .player
            .lock()
            .map_err(|_| "mpv embed state lock failed".to_string())?;

        if let Some(player) = player.as_ref() {
            player.host.resize()?;
        }

        Ok(())
    }
}

pub(super) fn stop_player(state: &MpvEmbedState) -> Result<(), String> {
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;

    if let Some(mut player) = player.take() {
        let _ = stop_recording_for_player(&mut player);
        player
            .mpv
            .command("stop", &[])
            .map_err(|error| format!("mpv stop failed: {error}"))?;
    }

    Ok(())
}

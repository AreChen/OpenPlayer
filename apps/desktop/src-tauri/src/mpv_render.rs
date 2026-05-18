use std::{path::PathBuf, sync::Mutex, sync::mpsc, time::Duration};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Serialize;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};

mod sys;
#[cfg(windows)]
mod win32_surface;

const COMMAND_RESPONSE_TIMEOUT: Duration = Duration::from_millis(750);
const DEFAULT_VOLUME: f64 = 82.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderViewport {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvRenderSnapshot {
    pub path: String,
    pub status: String,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
}

#[derive(Default)]
pub struct MpvRenderState {
    pub(crate) actor: Mutex<Option<MpvRenderActor>>,
}

#[allow(dead_code)]
enum RenderCommand {
    Open {
        path: String,
    },
    Play {
        response: mpsc::Sender<Result<MpvRenderSnapshot, String>>,
    },
    Pause {
        response: mpsc::Sender<Result<MpvRenderSnapshot, String>>,
    },
    Seek {
        position: f64,
        response: mpsc::Sender<Result<MpvRenderSnapshot, String>>,
    },
    SetVolume {
        volume: f64,
        response: mpsc::Sender<Result<MpvRenderSnapshot, String>>,
    },
    Snapshot {
        response: mpsc::Sender<Result<Option<MpvRenderSnapshot>, String>>,
    },
    Resize,
    Stop {
        response: mpsc::Sender<Result<(), String>>,
    },
    Shutdown,
}

pub(crate) struct MpvRenderActor {
    sender: mpsc::Sender<RenderCommand>,
}

impl Drop for MpvRenderActor {
    fn drop(&mut self) {
        let _ = self.sender.send(RenderCommand::Shutdown);
    }
}

impl MpvRenderState {
    pub fn start(&self, parent_hwnd: isize) -> Result<(), String> {
        let mut actor = self
            .actor
            .lock()
            .map_err(|_| "mpv render state lock failed".to_string())?;
        if actor.is_some() {
            return Ok(());
        }

        let (sender, receiver) = mpsc::channel::<RenderCommand>();
        std::thread::Builder::new()
            .name("openplayer-mpv-render".to_string())
            .spawn(move || render_thread(parent_hwnd, receiver))
            .map_err(|error| format!("failed to start mpv render thread: {error}"))?;

        *actor = Some(MpvRenderActor { sender });
        Ok(())
    }

    pub fn resize(&self) -> Result<(), String> {
        self.send_without_response(RenderCommand::Resize)
    }

    fn actor_sender(&self) -> Result<mpsc::Sender<RenderCommand>, String> {
        let actor = self
            .actor
            .lock()
            .map_err(|_| "mpv render state lock failed".to_string())?;
        actor
            .as_ref()
            .map(|actor| actor.sender.clone())
            .ok_or_else(|| "mpv render backend is not started".to_string())
    }

    fn send_without_response(&self, command: RenderCommand) -> Result<(), String> {
        self.actor_sender()?
            .send(command)
            .map_err(|_| "mpv render thread is unavailable".to_string())
    }
}

fn request_snapshot(
    state: &MpvRenderState,
    build: impl FnOnce(mpsc::Sender<Result<MpvRenderSnapshot, String>>) -> RenderCommand,
) -> Result<MpvRenderSnapshot, String> {
    let sender = state.actor_sender()?;
    let (response_tx, response_rx) = mpsc::channel();
    sender
        .send(build(response_tx))
        .map_err(|_| "mpv render thread is unavailable".to_string())?;
    response_rx
        .recv_timeout(COMMAND_RESPONSE_TIMEOUT)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => {
                "mpv render command timed out before the render thread responded".to_string()
            }
            mpsc::RecvTimeoutError::Disconnected => {
                "mpv render thread dropped command response".to_string()
            }
        })?
}

#[tauri::command]
pub fn mpv_render_open_path(
    window: tauri::Window,
    state: tauri::State<'_, MpvRenderState>,
    path: String,
) -> Result<MpvRenderSnapshot, String> {
    let path = validate_media_path(&path)?.to_string_lossy().to_string();
    let hwnd = window_hwnd(&window)?;
    state.start(hwnd as isize)?;
    state
        .inner()
        .actor_sender()?
        .send(RenderCommand::Open { path: path.clone() })
        .map_err(|_| "mpv render thread is unavailable".to_string())?;

    Ok(MpvRenderSnapshot {
        path,
        status: "opening".to_string(),
        paused: false,
        position: 0.0,
        duration: 0.0,
        volume: DEFAULT_VOLUME,
    })
}

#[tauri::command]
pub fn mpv_render_play(
    state: tauri::State<'_, MpvRenderState>,
) -> Result<MpvRenderSnapshot, String> {
    request_snapshot(state.inner(), |response| RenderCommand::Play { response })
}

#[tauri::command]
pub fn mpv_render_pause(
    state: tauri::State<'_, MpvRenderState>,
) -> Result<MpvRenderSnapshot, String> {
    request_snapshot(state.inner(), |response| RenderCommand::Pause { response })
}

#[tauri::command]
pub fn mpv_render_seek(
    state: tauri::State<'_, MpvRenderState>,
    position: f64,
) -> Result<MpvRenderSnapshot, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("invalid mpv seek target".to_string());
    }

    request_snapshot(state.inner(), |response| RenderCommand::Seek {
        position,
        response,
    })
}

#[tauri::command]
pub fn mpv_render_set_volume(
    state: tauri::State<'_, MpvRenderState>,
    volume: f64,
) -> Result<MpvRenderSnapshot, String> {
    let volume = clamp_volume(volume)?;
    request_snapshot(state.inner(), |response| RenderCommand::SetVolume {
        volume,
        response,
    })
}

#[tauri::command]
pub fn mpv_render_snapshot(
    state: tauri::State<'_, MpvRenderState>,
) -> Result<Option<MpvRenderSnapshot>, String> {
    let sender = state.actor_sender()?;
    let (response_tx, response_rx) = mpsc::channel();
    sender
        .send(RenderCommand::Snapshot {
            response: response_tx,
        })
        .map_err(|_| "mpv render thread is unavailable".to_string())?;
    response_rx
        .recv_timeout(COMMAND_RESPONSE_TIMEOUT)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => {
                "mpv render command timed out before the render thread responded".to_string()
            }
            mpsc::RecvTimeoutError::Disconnected => {
                "mpv render thread dropped command response".to_string()
            }
        })?
}

fn window_hwnd(window: &impl HasWindowHandle) -> Result<i64, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as i64),
        _ => Err("mpv render backend is only wired for Windows HWND targets".to_string()),
    }
}

pub(crate) fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("enter a local media path".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_file() {
        return Err(format!("media path does not exist: {}", path.display()));
    }

    Ok(path)
}

pub(crate) fn clamp_volume(volume: f64) -> Result<f64, String> {
    if !volume.is_finite() {
        return Err("invalid mpv volume".to_string());
    }

    Ok(volume.clamp(0.0, 100.0))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn render_viewport(width: i32, height: i32) -> RenderViewport {
    RenderViewport {
        x: 0,
        y: 0,
        width: width.max(1),
        height: height.max(1),
    }
}

#[cfg(windows)]
fn render_thread(parent_hwnd: isize, receiver: mpsc::Receiver<RenderCommand>) {
    let mut surface = match unsafe { win32_surface::Win32RenderSurface::new(parent_hwnd as _) } {
        Ok(surface) => surface,
        Err(error) => {
            eprintln!("mpv render surface init failed: {error}");
            return;
        }
    };
    if let Err(error) = surface.make_current() {
        eprintln!("mpv render make-current failed: {error}");
        return;
    }

    let mpv = match sys::RawMpv::new() {
        Ok(mpv) => mpv,
        Err(error) => {
            eprintln!("mpv init failed: {error}");
            return;
        }
    };
    let render = match mpv.create_render_context(win32_surface::get_proc_address) {
        Ok(render) => render,
        Err(error) => {
            eprintln!("mpv render context init failed: {error}");
            return;
        }
    };

    let (redraw_tx, redraw_rx) = mpsc::channel();
    render.set_update_callback(redraw_tx);
    let mut current_path = String::new();
    let mut current_volume = DEFAULT_VOLUME;

    loop {
        pump_render_thread_messages();

        while redraw_rx.try_recv().is_ok() {
            draw_frame(&render, &mut surface);
        }

        match receiver.recv_timeout(Duration::from_millis(16)) {
            Ok(RenderCommand::Open { path }) => {
                current_path = path.clone();
                if let Err(error) = mpv.command(&["loadfile", &path, "replace"]) {
                    eprintln!("mpv loadfile failed: {error}");
                }
                draw_frame(&render, &mut surface);
            }
            Ok(RenderCommand::Play { response }) => {
                let result = mpv
                    .set_flag_property("pause", false)
                    .map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::Pause { response }) => {
                let result = mpv
                    .set_flag_property("pause", true)
                    .map(|_| snapshot(&mpv, &current_path, current_volume, "paused"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::Seek { position, response }) => {
                let position = position.to_string();
                let result = mpv
                    .command(&["seek", &position, "absolute"])
                    .map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::SetVolume { volume, response }) => {
                current_volume = volume;
                let result = mpv
                    .set_double_property("volume", volume)
                    .map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::Snapshot { response }) => {
                let result = if current_path.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(snapshot(&mpv, &current_path, current_volume, "ready")))
                };
                let _ = response.send(result);
            }
            Ok(RenderCommand::Resize) => {
                let _ = surface.resize_to_parent();
                draw_frame(&render, &mut surface);
            }
            Ok(RenderCommand::Stop { response }) => {
                let result = mpv.command(&["stop"]).map(|_| ());
                let _ = response.send(result);
            }
            Ok(RenderCommand::Shutdown) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[cfg(windows)]
fn pump_render_thread_messages() {
    let mut message = MSG::default();
    while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(not(windows))]
fn render_thread(_parent_hwnd: isize, receiver: mpsc::Receiver<RenderCommand>) {
    for command in receiver {
        match command {
            RenderCommand::Open { .. } => {}
            RenderCommand::Play { response }
            | RenderCommand::Pause { response }
            | RenderCommand::Seek { response, .. }
            | RenderCommand::SetVolume { response, .. } => {
                let _ = response.send(Err(
                    "mpv render backend is only implemented on Windows".to_string()
                ));
            }
            RenderCommand::Snapshot { response } => {
                let _ = response.send(Err(
                    "mpv render backend is only implemented on Windows".to_string()
                ));
            }
            RenderCommand::Stop { response } => {
                let _ = response.send(Err(
                    "mpv render backend is only implemented on Windows".to_string()
                ));
            }
            RenderCommand::Resize => {}
            RenderCommand::Shutdown => break,
        }
    }
}

#[cfg(windows)]
fn draw_frame(render: &sys::RawRenderContext, surface: &mut win32_surface::Win32RenderSurface) {
    let _ = render.update();
    let viewport = surface.viewport();
    if render.render(viewport.width, viewport.height).is_ok() && surface.swap_buffers().is_ok() {
        render.report_swap();
    }
}

#[cfg(windows)]
fn snapshot(
    mpv: &sys::RawMpv,
    path: &str,
    volume: f64,
    fallback_status: &str,
) -> MpvRenderSnapshot {
    let paused = mpv.get_flag_property("pause");
    MpvRenderSnapshot {
        path: path.to_string(),
        status: if paused { "paused" } else { fallback_status }.to_string(),
        paused,
        position: mpv.get_double_property("time-pos"),
        duration: mpv.get_double_property("duration"),
        volume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_media_path() {
        let error = validate_media_path("   ").expect_err("empty paths should be rejected");

        assert_eq!(error, "enter a local media path");
    }

    #[test]
    fn clamps_valid_volume_to_mpv_percent_range() {
        assert_eq!(clamp_volume(-12.0).expect("finite volume is valid"), 0.0);
        assert_eq!(clamp_volume(42.5).expect("finite volume is valid"), 42.5);
        assert_eq!(clamp_volume(182.0).expect("finite volume is valid"), 100.0);
    }

    #[test]
    fn rejects_non_finite_volume() {
        let error = clamp_volume(f64::NAN).expect_err("NaN volume should be rejected");

        assert_eq!(error, "invalid mpv volume");
    }

    #[test]
    fn render_viewport_fills_available_area_without_control_reserves() {
        assert_eq!(
            render_viewport(1280, 720),
            RenderViewport {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            }
        );
    }
}

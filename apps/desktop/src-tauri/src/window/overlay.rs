#[cfg(feature = "mpv-embed")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time::Duration};

#[cfg(feature = "mpv-embed")]
use crate::mpv_embed::{MpvEmbedState, stop_embedded_player_for_close};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size};
#[cfg(feature = "mpv-embed")]
use tauri::{WebviewUrl, WebviewWindowBuilder, WindowEvent, utils::config::Color};

use super::{
    MIN_MAIN_WINDOW_HEIGHT, MIN_MAIN_WINDOW_WIDTH, WindowState, begin_window_close, main_window,
    overlay_platform::prepare_macos_main_window_chrome, overlay_platform::set_overlay_owner,
    overlay_window,
};

#[cfg(feature = "mpv-embed")]
static MPV_VIDEO_HOST_SYNC_PENDING: AtomicBool = AtomicBool::new(false);

pub(super) fn sync_overlay_to_main(app: &AppHandle) {
    sync_overlay_to_main_with_focus(app, true);
}

fn sync_overlay_to_main_without_focus(app: &AppHandle) {
    sync_overlay_to_main_with_focus(app, false);
}

pub(super) fn sync_overlay_to_main_after_resize(app: &AppHandle) {
    sync_overlay_to_main_without_focus(app);
}

#[cfg(target_os = "macos")]
fn sync_main_to_overlay_after_resize(app: &AppHandle) {
    let Ok(main) = main_window(app) else {
        return;
    };
    if main.is_fullscreen().unwrap_or(false) || main.is_maximized().unwrap_or(false) {
        return;
    }
    let Some(overlay) = overlay_window(app) else {
        return;
    };
    let Ok(position) = overlay.outer_position() else {
        return;
    };
    let Ok(size) = overlay.outer_size() else {
        return;
    };
    let _ = main.set_position(Position::Physical(PhysicalPosition {
        x: position.x,
        y: position.y,
    }));
    let _ = main.set_size(Size::Physical(PhysicalSize {
        width: size.width,
        height: size.height,
    }));
    #[cfg(feature = "mpv-embed")]
    schedule_mpv_video_host_sync(app);
}

fn sync_overlay_to_main_with_focus(app: &AppHandle, focus_overlay: bool) {
    let Ok(main) = main_window(app) else {
        return;
    };
    let Some(overlay) = overlay_window(app) else {
        return;
    };
    let Ok(position) = main.outer_position() else {
        return;
    };
    let Ok(size) = main.outer_size() else {
        return;
    };
    let _ = overlay.set_position(Position::Physical(PhysicalPosition {
        x: position.x,
        y: position.y,
    }));
    let _ = overlay.set_size(Size::Physical(PhysicalSize {
        width: size.width,
        height: size.height,
    }));
    if focus_overlay {
        focus_overlay_window(app);
    }
}

pub(super) fn focus_overlay_window(app: &AppHandle) {
    if let Some(overlay) = overlay_window(app) {
        let _ = overlay.set_focus();
    }
}

pub(super) fn schedule_overlay_sync_to_main(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        for delay in [
            Duration::from_millis(40),
            Duration::from_millis(120),
            Duration::from_millis(260),
        ] {
            thread::sleep(delay);
            let app_for_sync = app.clone();
            let _ = app.run_on_main_thread(move || sync_overlay_to_main(&app_for_sync));
        }
    });
}

#[cfg(feature = "mpv-embed")]
fn sync_mpv_video_host(app: &AppHandle) {
    let state = app.state::<MpvEmbedState>();
    let _ = state.resize_video_host();
}

#[cfg(feature = "mpv-embed")]
fn schedule_mpv_video_host_sync(app: &AppHandle) {
    if MPV_VIDEO_HOST_SYNC_PENDING.swap(true, Ordering::SeqCst) {
        return;
    }

    let app = app.clone();
    thread::spawn(move || {
        for delay in [
            Duration::from_millis(16),
            Duration::from_millis(80),
            Duration::from_millis(180),
        ] {
            thread::sleep(delay);
            let app_for_sync = app.clone();
            let _ = app.run_on_main_thread(move || sync_mpv_video_host(&app_for_sync));
        }
        MPV_VIDEO_HOST_SYNC_PENDING.store(false, Ordering::SeqCst);
    });
}

#[cfg(feature = "mpv-embed")]
pub(crate) fn setup_overlay_window(app: &mut tauri::App) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        prepare_macos_main_window_chrome(&window);
        let overlay = WebviewWindowBuilder::new(
            app,
            "overlay",
            WebviewUrl::App("index.html?surface=overlay".into()),
        )
        .title("OpenPlayer Controls")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(cfg!(target_os = "macos"))
        .min_inner_size(MIN_MAIN_WINDOW_WIDTH as f64, MIN_MAIN_WINDOW_HEIGHT as f64)
        .skip_taskbar(true)
        .background_color(Color(0, 0, 0, 0))
        .visible(false)
        .build()
        .map_err(|error| format!("failed to create overlay controls window: {error}"))?;
        let _ = overlay.set_background_color(Some(Color(0, 0, 0, 0)));
        set_overlay_owner(&window, &overlay);

        let app_handle = app.handle().clone();
        sync_overlay_to_main(&app_handle);
        let _ = overlay.show();
        set_overlay_owner(&window, &overlay);
        let app_handle_for_overlay = app_handle.clone();
        overlay.on_window_event(move |event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                let window_state = app_handle_for_overlay.state::<WindowState>();
                if !begin_window_close(window_state.inner()) {
                    return;
                }

                let _ = stop_embedded_player_for_close(&app_handle_for_overlay);
                let Ok(main) = main_window(&app_handle_for_overlay) else {
                    return;
                };
                let _ = main.close();
            }
            #[cfg(target_os = "macos")]
            if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                sync_main_to_overlay_after_resize(&app_handle_for_overlay);
            }
        });
        window.on_window_event(move |event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                let window_state = app_handle.state::<WindowState>();
                if begin_window_close(window_state.inner()) {
                    let _ = stop_embedded_player_for_close(&app_handle);
                    if let Some(overlay) = overlay_window(&app_handle) {
                        let _ = overlay.close();
                    }
                }
            }
            if matches!(
                event,
                WindowEvent::Moved(_)
                    | WindowEvent::Resized(_)
                    | WindowEvent::ScaleFactorChanged { .. }
            ) {
                sync_overlay_to_main_without_focus(&app_handle);
                schedule_mpv_video_host_sync(&app_handle);
            }
            if matches!(event, WindowEvent::Focused(true)) {
                focus_overlay_window(&app_handle);
            }
        });
    }

    Ok(())
}

use super::{WindowState, chrome, main_window, overlay, overlay_window};
use crate::mpv_embed::{MpvEmbedState, MpvWallState, mpv_embed_snapshot};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, PhysicalSize, RunEvent, WebviewUrl};

pub fn run() {
    let close_target = std::env::args().nth(1).unwrap_or_else(|| "overlay".into());
    assert!(matches!(close_target.as_str(), "main" | "overlay"));
    let directory =
        std::env::temp_dir().join(format!("openplayer-window-smoke-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let video = directory.join("fixture.y4m");
    write_video(&video);
    let close_started = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let watchdog = finished.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(45));
        if !watchdog.load(Ordering::SeqCst) {
            eprintln!("window smoke timed out (native event loop or teardown stalled)");
            std::process::exit(1);
        }
    });
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = format!("dev.openplayer.smoke.p{}", std::process::id());
    context.config_mut().build.dev_url = None;
    context.config_mut().app.windows[0].url = WebviewUrl::External("about:blank".parse().unwrap());
    context.config_mut().app.windows[0].data_directory = Some(directory.join("webview"));
    let close_for_setup = close_started.clone();
    let app = tauri::Builder::default()
        .manage(WindowState::default())
        .manage(MpvEmbedState::default())
        .manage(MpvWallState::default())
        .setup(move |app| {
            crate::native_shortcuts::install_native_shortcut_hook(app.handle().clone());
            // Real native windows with inert WebViews: no user stores or plugins are loaded.
            overlay::setup_overlay_window_with_url(
                app,
                WebviewUrl::External("about:blank".parse().unwrap()),
            )?;
            let handle = app.handle().clone();
            thread::spawn(move || {
                if let Err(error) = exercise_windows(&handle, video, close_target, &close_for_setup)
                {
                    eprintln!("window smoke failed: {error}");
                    std::process::exit(1);
                }
            });
            Ok(())
        })
        .build(context)
        .expect("window smoke app should start");
    app.run(move |app, event| {
        if matches!(event, RunEvent::Exit) {
            let player = tauri::async_runtime::block_on(mpv_embed_snapshot(app.clone()));
            if !close_started.load(Ordering::SeqCst)
                || !app.webview_windows().is_empty()
                || !matches!(player, Ok(None))
            {
                eprintln!("window smoke exited without closing both windows and releasing mpv");
                std::process::exit(1);
            }
            finished.store(true, Ordering::SeqCst);
            println!("PASS: resize alignment, fullscreen restore, companion close, mpv teardown");
        }
    });
    let _ = fs::remove_dir_all(directory);
}

fn on_main<T: Send + 'static>(
    app: &AppHandle,
    action: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(action(&handle));
    })
    .map_err(|error| error.to_string())?;
    receiver
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| "native main thread is unresponsive".to_string())?
}

fn wait_until(
    app: &AppHandle,
    label: &str,
    check: impl Fn(&AppHandle) -> Result<bool, String> + Send + Sync + 'static,
) -> Result<(), String> {
    let check = Arc::new(check);
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        let check = check.clone();
        if on_main(app, move |app| check(app))? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!("timed out waiting for {label}"))
}

fn aligned(app: &AppHandle) -> Result<bool, String> {
    let main = main_window(app)?;
    let overlay = overlay_window(app).ok_or("overlay missing")?;
    Ok(main.outer_size().map_err(|e| e.to_string())?
        == overlay.outer_size().map_err(|e| e.to_string())?
        && main.outer_position().map_err(|e| e.to_string())?
            == overlay.outer_position().map_err(|e| e.to_string())?)
}

#[cfg(windows)]
fn fullscreen_covers_monitor(app: &AppHandle) -> Result<bool, String> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{FindWindowExW, GetWindowRect},
    };
    let main = main_window(app)?;
    let monitor = main
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("monitor missing")?;
    let position = *monitor.position();
    let size = *monitor.size();
    let hwnd = main.hwnd().map_err(|e| e.to_string())?;
    let title: Vec<u16> = "OpenPlayer MPV Video Host"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let host = unsafe {
        FindWindowExW(
            hwnd.0 as _,
            std::ptr::null_mut(),
            std::ptr::null(),
            title.as_ptr(),
        )
    };
    let mut rect = RECT::default();
    if host.is_null() || unsafe { GetWindowRect(host, &mut rect) } == 0 {
        return Err("mpv video host bounds unavailable".into());
    }
    Ok(main.is_fullscreen().map_err(|e| e.to_string())?
        && main.inner_position().map_err(|e| e.to_string())? == position
        && main.inner_size().map_err(|e| e.to_string())? == size
        && rect.left == position.x
        && rect.top == position.y
        && rect.right - rect.left == size.width as i32
        && rect.bottom - rect.top == size.height as i32
        && aligned(app)?)
}

#[cfg(windows)]
fn exercise_maximized_fullscreen(app: &AppHandle) -> Result<(), String> {
    let original = on_main(app, |app| {
        let main = main_window(app)?;
        Ok((
            main.outer_position().map_err(|e| e.to_string())?,
            main.outer_size().map_err(|e| e.to_string())?,
        ))
    })?;
    on_main(app, |app| chrome::toggle_maximize(app.clone()))?;
    wait_until(app, "maximize", |app| {
        Ok(main_window(app)?
            .is_maximized()
            .map_err(|e| e.to_string())?
            && aligned(app)?)
    })?;
    for _ in 0..2 {
        on_main(app, |app| {
            chrome::toggle_fullscreen(app.clone(), &app.state::<WindowState>())
        })?;
        if let Err(error) = wait_until(
            app,
            "maximized fullscreen covers entire monitor",
            fullscreen_covers_monitor,
        ) {
            let bounds = on_main(app, |app| {
                let main = main_window(app)?;
                Ok(format!(
                    "outer={:?}, client={:?}, maximized={:?}, monitor={:?}",
                    main.outer_size(),
                    main.inner_size(),
                    main.is_maximized(),
                    main.current_monitor().map(|m| m.map(|m| *m.size()))
                ))
            })?;
            return Err(format!("{error}; {bounds}"));
        }
        on_main(app, |app| {
            chrome::toggle_fullscreen(app.clone(), &app.state::<WindowState>())
        })?;
        wait_until(app, "restore maximized state", |app| {
            let main = main_window(app)?;
            Ok(!main.is_fullscreen().map_err(|e| e.to_string())?
                && main.is_maximized().map_err(|e| e.to_string())?
                && aligned(app)?)
        })?;
    }
    on_main(app, |app| chrome::toggle_maximize(app.clone()))?;
    wait_until(app, "unmaximize after fullscreen", move |app| {
        let main = main_window(app)?;
        Ok(!main.is_maximized().map_err(|e| e.to_string())?
            && main.outer_position().map_err(|e| e.to_string())? == original.0
            && main.outer_size().map_err(|e| e.to_string())? == original.1
            && aligned(app)?)
    })
}

#[cfg(windows)]
fn exercise_capture_mode(app: &AppHandle) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    wait_until(app, "native capture recovery hook", |_| {
        Ok(crate::native_shortcuts::capture_recovery_available())
    })?;
    tauri::async_runtime::block_on(super::window_set_capture_mode(app.clone(), true))?;
    wait_until(app, "capture source focus", |app| {
        let main = main_window(app)?;
        let overlay = overlay_window(app).ok_or("overlay missing")?;
        Ok(super::capture_mode_active(app)
            && !overlay.is_visible().map_err(|e| e.to_string())?
            && std::ptr::eq(
                unsafe { GetForegroundWindow() },
                main.hwnd().map_err(|e| e.to_string())?.0,
            ))
    })?;
    on_main(app, |app| {
        chrome::focus_overlay(app.clone())?;
        overlay::sync_overlay_to_main(app);
        if crate::native_shortcuts::recover_capture_on_escape(app, 0x1B, false)
            || crate::native_shortcuts::recover_capture_on_escape(app, 0x20, true)
        {
            return Err("capture recovery intercepted unrelated input".into());
        }
        let main = main_window(app)?;
        if !std::ptr::eq(
            unsafe { GetForegroundWindow() },
            main.hwnd().map_err(|e| e.to_string())?.0,
        ) {
            return Err("hidden overlay stole capture source focus".into());
        }
        Ok(())
    })?;
    crate::native_shortcuts::window_set_shortcuts_enabled(false)?;
    if !crate::native_shortcuts::recover_capture_on_escape(app, 0x1B, true) {
        return Err("native Escape did not restore capture controls".into());
    }
    wait_until(app, "capture controls restored", |app| {
        let overlay = overlay_window(app).ok_or("overlay missing")?;
        Ok(!super::capture_mode_active(app)
            && overlay.is_visible().map_err(|e| e.to_string())?
            && std::ptr::eq(
                unsafe { GetForegroundWindow() },
                overlay.hwnd().map_err(|e| e.to_string())?.0,
            ))
    })?;
    crate::native_shortcuts::window_set_shortcuts_enabled(true)?;
    println!(
        "PASS: maximized fullscreen client/video bounds, capture source focus, native recovery"
    );
    Ok(())
}

fn exercise_windows(
    app: &AppHandle,
    video: std::path::PathBuf,
    target: String,
    close_started: &AtomicBool,
) -> Result<(), String> {
    wait_until(app, "initial alignment", aligned)?;
    on_main(app, move |app| {
        super::mpv_overlay_open_path(
            app.clone(),
            app.state(),
            video.to_string_lossy().into_owned(),
            None,
            Some(0.0),
            None,
        )
        .map(|_| ())
    })?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = tauri::async_runtime::block_on(mpv_embed_snapshot(app.clone()))?;
        if snapshot.is_some_and(|snapshot| snapshot.duration > 0.0 && snapshot.position > 0.0) {
            break;
        }
        if Instant::now() > deadline {
            return Err("mpv fixture did not start playing".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
    for index in 0..40 {
        on_main(app, move |app| {
            let window = if cfg!(target_os = "macos") {
                overlay_window(app).ok_or("overlay missing")?
            } else {
                main_window(app)?
            };
            window
                .set_size(PhysicalSize::new(1000 + index * 4, 600 + index * 2))
                .map_err(|e| e.to_string())
        })?;
        thread::sleep(Duration::from_millis(16));
    }
    wait_until(app, "resize alignment", aligned)?;
    let placement = on_main(app, |app| {
        let main = main_window(app)?;
        Ok((
            main.outer_position().map_err(|e| e.to_string())?,
            main.outer_size().map_err(|e| e.to_string())?,
        ))
    })?;
    on_main(app, |app| {
        chrome::toggle_fullscreen(app.clone(), &app.state::<WindowState>())
    })?;
    wait_until(app, "fullscreen entry", |app| {
        main_window(app)?.is_fullscreen().map_err(|e| e.to_string())
    })?;
    wait_until(app, "fullscreen alignment", aligned)?;
    on_main(app, |app| {
        chrome::toggle_fullscreen(app.clone(), &app.state::<WindowState>())
    })?;
    let restore_result = wait_until(app, "fullscreen restore", move |app| {
        let main = main_window(app)?;
        Ok(!main.is_fullscreen().map_err(|e| e.to_string())?
            && main.outer_position().map_err(|e| e.to_string())? == placement.0
            && main.outer_size().map_err(|e| e.to_string())? == placement.1
            && aligned(app)?)
    });
    if let Err(error) = restore_result {
        let actual = on_main(app, |app| {
            let main = main_window(app)?;
            let overlay = overlay_window(app).ok_or("overlay missing")?;
            Ok(format!(
                "main={:?}/{:?}, overlay={:?}/{:?}, fullscreen={:?}",
                main.outer_position(),
                main.outer_size(),
                overlay.outer_position(),
                overlay.outer_size(),
                main.is_fullscreen()
            ))
        })?;
        return Err(format!("{error}; expected={placement:?}; {actual}"));
    }
    #[cfg(windows)]
    exercise_maximized_fullscreen(app)?;
    #[cfg(windows)]
    exercise_capture_mode(app)?;
    #[cfg(windows)]
    if target == "main" {
        tauri::async_runtime::block_on(super::window_set_capture_mode(app.clone(), true))?;
    }
    close_started.store(true, Ordering::SeqCst);
    on_main(app, move |app| {
        let window = app
            .get_webview_window(&target)
            .ok_or("close target missing")?;
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetForegroundWindow, PostMessageW, WM_CLOSE,
            };
            let handle = window.hwnd().map_err(|error| error.to_string())?;
            if target == "main" {
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_F4,
                    VK_MENU,
                };
                if !std::ptr::eq(unsafe { GetForegroundWindow() }, handle.0) {
                    return Err("refusing to inject Alt+F4 outside the smoke window".into());
                }
                let inputs = [
                    (VK_MENU, 0),
                    (VK_F4, 0),
                    (VK_F4, KEYEVENTF_KEYUP),
                    (VK_MENU, KEYEVENTF_KEYUP),
                ]
                .map(|(key, flags)| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: key,
                            dwFlags: flags,
                            ..Default::default()
                        },
                    },
                });
                if unsafe {
                    SendInput(
                        inputs.len() as u32,
                        inputs.as_ptr(),
                        std::mem::size_of::<INPUT>() as i32,
                    )
                } != inputs.len() as u32
                {
                    return Err("failed to send native Alt+F4".into());
                }
            } else if unsafe { PostMessageW(handle.0 as _, WM_CLOSE, 0, 0) } == 0 {
                return Err("failed to send native close request".into());
            }
            Ok(())
        }
        #[cfg(not(windows))]
        window.close().map_err(|error| error.to_string())
    })
}

fn write_video(path: &Path) {
    let mut file = std::io::BufWriter::new(fs::File::create(path).unwrap());
    file.write_all(b"YUV4MPEG2 W64 H48 F30:1 Ip A1:1 C420jpeg\n")
        .unwrap();
    for index in 0..1800 {
        file.write_all(b"FRAME\n").unwrap();
        file.write_all(&vec![32 + (index % 190) as u8; 64 * 48])
            .unwrap();
        file.write_all(&[128; 64 * 48 / 2]).unwrap();
    }
}

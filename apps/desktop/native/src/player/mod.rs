use std::{cell::RefCell, path::PathBuf, rc::Rc};

use anyhow::{Context, Result, bail};
#[cfg(windows)]
use gpui::Window;
use gpui::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
#[cfg(windows)]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod browser_options;
mod controller;
mod locale;
mod media_info;
mod panels;
mod player_view;
mod render_context;
mod settings;
mod theme;
mod transport;
mod ui_state;
#[cfg(unix)]
mod unix_window;
mod updates;
#[cfg(windows)]
mod video;
#[cfg(unix)]
#[path = "video_unix.rs"]
mod video;
mod widgets;
mod window_layout;

use self::{controller::Controller, player_view::PlayerView, video::VideoRenderer};

const WINDOW_TITLE: &str = "OpenPlayer";

#[derive(Default)]
struct Options {
    media: Vec<PathBuf>,
    adapter: Option<u32>,
    update_smoke: Option<updates::UpdateSmoke>,
}

impl Options {
    fn parse() -> Result<Option<Self>> {
        let mut args = std::env::args_os().skip(1);
        let mut options = Self::default();
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--help" | "-h") => {
                    #[cfg(windows)]
                    println!("openplayer-desktop [--adapter DXGI_INDEX] [MEDIA ...]");
                    #[cfg(unix)]
                    println!("openplayer-desktop [MEDIA ...]");
                    return Ok(None);
                }
                Some("--version" | "-V") => {
                    println!("OpenPlayer {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                #[cfg(windows)]
                Some("--adapter") => {
                    let value = args.next().context("--adapter requires a DXGI index")?;
                    options.adapter = Some(
                        value
                            .to_string_lossy()
                            .parse()
                            .context("invalid DXGI index")?,
                    );
                }
                Some("--smoke-update") => {
                    let action = args.next().context("missing smoke update action")?;
                    let status = args.next().context("missing smoke update status path")?;
                    options.update_smoke = Some(updates::UpdateSmoke::new(
                        action.to_str(),
                        PathBuf::from(status),
                    )?);
                }
                Some(flag) if flag.starts_with('-') => bail!("unknown option {flag}"),
                _ => options.media.push(PathBuf::from(arg)),
            }
        }
        if let Some(smoke) = &mut options.update_smoke {
            smoke.wait_for_playback = !options.media.is_empty();
        }
        Ok(Some(options))
    }
}

pub fn run() {
    if let Err(error) = run_player() {
        eprintln!("OpenPlayer startup failed: {error:#}");
        #[cfg(windows)]
        {
            let message = format!("{error:#}")
                .encode_utf16()
                .chain([0])
                .collect::<Vec<u16>>();
            let title = WINDOW_TITLE.encode_utf16().chain([0]).collect::<Vec<u16>>();
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MessageBoxW};
                MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    MB_ICONERROR,
                );
            }
        }
        std::process::exit(1);
    }
}

fn run_player() -> Result<()> {
    let Some(options) = Options::parse()? else {
        return Ok(());
    };
    let launch_error = Rc::new(RefCell::new(None));
    let result = launch_error.clone();

    let platform = gpui_platform::current_platform(false);
    let app = gpui::Application::with_platform(platform.clone()).with_assets(theme::Assets);
    #[cfg(target_os = "macos")]
    let mut opened_files = {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        app.on_open_urls(move |urls| {
            let paths: Vec<_> = urls
                .iter()
                .filter_map(|value| url::Url::parse(value).ok()?.to_file_path().ok())
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            if !paths.is_empty() {
                let _ = tx.send(paths);
            }
        });
        rx
    };
    app.run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        let window = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                #[cfg(target_os = "macos")]
                app_owns_titlebar_drag: true,
                window_min_size: Some(window_layout::minimum_size()),
                titlebar: Some(TitlebarOptions {
                    title: Some(WINDOW_TITLE.into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                #[cfg(target_os = "macos")]
                window.set_native_titlebar_visibility(false);
                let controller = Controller::new(options.media);
                cx.new(|cx| {
                    PlayerView::new(
                        options.adapter,
                        controller,
                        platform,
                        options.update_smoke,
                        window,
                        cx,
                    )
                })
            },
        );
        if let Err(error) = window {
            *result.borrow_mut() = Some(error);
            cx.quit();
            return;
        }
        #[cfg(target_os = "macos")]
        if let Ok(handle) = window {
            cx.spawn(async move |cx| {
                while let Some(paths) = opened_files.recv().await {
                    if handle
                        .update(cx, |view, _, cx| {
                            view.send("paths", serde_json::json!(paths), cx)
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.activate(true);
    });

    match launch_error.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(windows)]
fn hwnd(window: &Window) -> Result<isize> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("OpenPlayer window handle is unavailable: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        _ => bail!("OpenPlayer requires a Win32 window handle"),
    }
}

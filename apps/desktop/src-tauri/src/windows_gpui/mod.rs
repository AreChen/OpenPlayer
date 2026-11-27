use std::{cell::RefCell, path::PathBuf, rc::Rc};

use anyhow::{Context, Result, bail};
use gpui::{
    App, AppContext, Bounds, TitlebarOptions, Window, WindowBounds, WindowOptions, px, size,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod browser_options;
mod controller;
mod locale;
mod media_info;
mod panels;
mod player_view;
mod settings;
mod theme;
mod transport;
mod ui_state;
mod updates;
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
                    println!("openplayer-desktop [--adapter DXGI_INDEX] [MEDIA ...]");
                    return Ok(None);
                }
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
        let message = format!("{error:#}")
            .encode_utf16()
            .chain([0])
            .collect::<Vec<u16>>();
        let title = WINDOW_TITLE.encode_utf16().chain([0]).collect::<Vec<u16>>();
        eprintln!("OpenPlayer startup failed: {error:#}");
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
}

fn run_player() -> Result<()> {
    let Some(options) = Options::parse()? else {
        return Ok(());
    };
    let launch_error = Rc::new(RefCell::new(None));
    let result = launch_error.clone();

    let platform = gpui_platform::current_platform(false);
    gpui::Application::with_platform(platform.clone())
        .with_assets(theme::Assets)
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(window_layout::minimum_size()),
                    titlebar: Some(TitlebarOptions {
                        title: Some(WINDOW_TITLE.into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
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
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            cx.activate(true);
        });

    match launch_error.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn hwnd(window: &Window) -> Result<isize> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("OpenPlayer window handle is unavailable: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        _ => bail!("OpenPlayer requires a Win32 window handle"),
    }
}

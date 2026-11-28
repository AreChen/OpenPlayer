use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub(super) fn set_above(window: &Window, above: bool) -> anyhow::Result<()> {
    let handle = HasWindowHandle::window_handle(window)?.as_raw();
    set_level(handle, above)
}

#[cfg(target_os = "macos")]
fn set_level(handle: RawWindowHandle, above: bool) -> anyhow::Result<()> {
    use objc2::{msg_send, runtime::AnyObject};
    let RawWindowHandle::AppKit(native) = handle else {
        anyhow::bail!("missing AppKit window")
    };
    // GPUI's NSView and its owning NSWindow are used only on the UI thread.
    unsafe {
        let window: *mut AnyObject = msg_send![native.ns_view.cast::<AnyObject>().as_ref(), window];
        let window = window
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("view has no window"))?;
        let level: isize = if above { 3 } else { 0 }; // NSFloatingWindowLevel / NSNormalWindowLevel
        let _: () = msg_send![window, setLevel: level];
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_level(handle: RawWindowHandle, above: bool) -> anyhow::Result<()> {
    use x11rb::{
        connection::Connection,
        protocol::xproto::{ClientMessageEvent, ConnectionExt, EventMask},
    };
    let xid = match handle {
        RawWindowHandle::Xcb(h) => h.window.get(),
        RawWindowHandle::Xlib(h) => u32::try_from(h.window)?,
        _ => anyhow::bail!("this compositor manages window stacking"),
    };
    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen].root;
    let state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let above_atom = conn
        .intern_atom(false, b"_NET_WM_STATE_ABOVE")?
        .reply()?
        .atom;
    let message = ClientMessageEvent::new(32, xid, state, [u32::from(above), above_atom, 0, 1, 0]);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        message,
    )?
    .check()?;
    conn.flush()?;
    Ok(())
}

#[cfg(feature = "mpv-embed")]
mod embedded;
#[cfg(not(feature = "mpv-embed"))]
mod fallback;

#[cfg(feature = "mpv-embed")]
pub use embedded::run;
#[cfg(not(feature = "mpv-embed"))]
pub use fallback::run;

// Expand once: macOS debug contexts embed a process-wide Info.plist symbol.
pub(crate) fn context() -> tauri::Context<tauri::Wry> {
    tauri::generate_context!()
}

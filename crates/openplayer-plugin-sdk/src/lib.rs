#![cfg_attr(not(feature = "manifest"), no_std)]

#[cfg(feature = "manifest")]
mod manifest;
#[cfg(feature = "manifest")]
pub use manifest::*;

pub const API_VERSION: u32 = 2;

/// Snapshot taken when the user invokes a plugin action.
#[derive(Clone, Copy, Debug)]
pub struct PlaybackContext {
    pub loaded: bool,
    pub paused: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub subtitle_scale: f64,
}

impl Default for PlaybackContext {
    fn default() -> Self {
        Self {
            loaded: false,
            paused: false,
            position_ms: 0,
            duration_ms: 0,
            subtitle_scale: 1.0,
        }
    }
}

impl PlaybackContext {
    #[doc(hidden)]
    pub fn flags(self) -> u32 {
        u32::from(self.loaded) | (u32::from(self.paused) << 1)
    }
}

/// Export the SDK 2 entry point from a Rust `cdylib` targeting
/// `wasm32-unknown-unknown`. The callback receives the manifest action index
/// and playback state, and selects the manifest action whose command will run.
#[macro_export]
macro_rules! export_plugin {
    ($invoke:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn openplayer_invoke_v2(
            action: u32,
            flags: u32,
            position_ms: u64,
            duration_ms: u64,
            subtitle_scale: f64,
        ) -> i32 {
            let context = $crate::PlaybackContext {
                loaded: flags & 1 != 0,
                paused: flags & 2 != 0,
                position_ms,
                duration_ms,
                subtitle_scale,
            };
            let selected: Option<u32> = $invoke(action, context);
            selected
                .and_then(|index| i32::try_from(index).ok())
                .unwrap_or(-1)
        }
    };
}

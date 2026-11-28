//! Open the stream dialog while idle. Save a screenshot during playback.
use openplayer_plugin_sdk::{PlaybackContext, export_plugin};

fn invoke(action: u32, state: PlaybackContext) -> Option<u32> {
    match action {
        0 if state.loaded => Some(0),
        0 | 1 => Some(1),
        _ => None,
    }
}

export_plugin!(invoke);

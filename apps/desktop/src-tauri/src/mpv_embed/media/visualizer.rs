use super::*;

pub(in crate::mpv_embed) fn configure_audio_visualizer(mpv: &libmpv2::Mpv, path: &Path) {
    if !is_likely_audio_path(path) {
        return;
    }

    if let Err(error) = mpv.set_property("audio-display", "no") {
        eprintln!("OpenPlayer mpv audio visualizer: failed to disable cover art: {error}");
    }
}

pub(in crate::mpv_embed) fn is_likely_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            AUDIO_VISUALIZER_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .unwrap_or(false)
}

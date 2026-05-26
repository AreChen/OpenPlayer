use super::super::*;

pub(in crate::mpv_embed) fn request_mpv_log_messages(mpv: &libmpv2::Mpv) {
    let Ok(min_level) = CString::new("v") else {
        return;
    };
    let result =
        unsafe { libmpv2_sys::mpv_request_log_messages(mpv.ctx.as_ptr(), min_level.as_ptr()) };
    if result < 0 {
        eprintln!("OpenPlayer mpv log subscription failed: {result}");
    }
}

pub(in crate::mpv_embed) fn log_selected_mpv_video_output_config(config: &MpvVideoOutputConfig) {
    eprintln!(
        "OpenPlayer mpv video output: vo={}, gpu-context={}, hwdec={}",
        config.vo.as_deref().unwrap_or("mpv-default"),
        config.gpu_context.as_deref().unwrap_or("mpv-default"),
        config.hwdec
    );
}

pub(in crate::mpv_embed) fn log_mpv_video_diagnostic(prefix: &str, level: &str, text: &str) {
    if is_mpv_video_diagnostic_log(level, prefix, text) {
        eprintln!(
            "OpenPlayer mpv {level}/{prefix}: {}",
            text.trim_end_matches(['\r', '\n'])
        );
    }
}

pub(in crate::mpv_embed) fn is_mpv_video_diagnostic_log(
    level: &str,
    prefix: &str,
    text: &str,
) -> bool {
    let level = level.to_ascii_lowercase();
    if matches!(level.as_str(), "fatal" | "error" | "warn") {
        return true;
    }

    let prefix = prefix.to_ascii_lowercase();
    if prefix.starts_with("vo") || matches!(prefix.as_str(), "vd" | "ffmpeg/video") {
        return true;
    }

    let text = text.to_ascii_lowercase();
    text.contains("vo:")
        || text.contains("[vo")
        || text.contains("gpu")
        || text.contains("egl")
        || text.contains("dri")
        || text.contains("vaapi")
        || text.contains("vdpau")
        || text.contains("hwdec")
}

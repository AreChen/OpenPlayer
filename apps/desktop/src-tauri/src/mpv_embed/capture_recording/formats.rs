use super::super::*;

pub(in crate::mpv_embed) fn normalize_capture_image_format(
    format: Option<String>,
) -> Result<String, String> {
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("png")
        .to_ascii_lowercase();
    match format.as_str() {
        "png" | "jpg" | "webp" => Ok(format),
        "jpeg" => Ok("jpg".to_string()),
        _ => Err(format!("unsupported screenshot format: {format}")),
    }
}

pub(in crate::mpv_embed) fn normalize_recording_container_format(
    format: Option<String>,
) -> Result<String, String> {
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    match format.as_str() {
        "mp4" | "mkv" | "ts" => Ok(format),
        _ => Err(format!("unsupported recording format: {format}")),
    }
}

pub(in crate::mpv_embed) fn recording_container_format_for_method(
    method: &MpvRecordingMethod,
    requested_format: &str,
) -> String {
    match method {
        MpvRecordingMethod::DumpCache { .. } | MpvRecordingMethod::StreamRecord => {
            requested_format.to_string()
        }
    }
}

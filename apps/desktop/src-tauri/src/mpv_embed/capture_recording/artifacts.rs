use super::super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

const MAX_FRAME_CAPTURE_BASE64_BYTES: u64 = 2 * 1024 * 1024;

pub(in crate::mpv_embed) fn frame_capture_artifact(
    output_path: &Path,
    format: &str,
    include_base64: bool,
) -> Result<MpvFrameCaptureArtifact, String> {
    let metadata = fs::metadata(output_path)
        .map_err(|error| format!("frame capture output was not created: {error}"))?;
    if metadata.len() == 0 {
        let _ = fs::remove_file(output_path);
        return Err("mpv produced an empty frame capture".to_string());
    }

    let size_bytes = metadata.len();
    let body_base64 = if include_base64 {
        if size_bytes > MAX_FRAME_CAPTURE_BASE64_BYTES {
            return Err(format!(
                "frame capture is too large to return as base64: {size_bytes} bytes exceeds {MAX_FRAME_CAPTURE_BASE64_BYTES}"
            ));
        }
        Some(
            BASE64_STANDARD.encode(
                fs::read(output_path)
                    .map_err(|error| format!("failed to read frame capture output: {error}"))?,
            ),
        )
    } else {
        None
    };

    Ok(MpvFrameCaptureArtifact {
        path: output_path.to_string_lossy().to_string(),
        format: format.to_string(),
        mime_type: frame_capture_mime_type(format).to_string(),
        size_bytes,
        body_base64,
    })
}

fn frame_capture_mime_type(format: &str) -> &'static str {
    match format {
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

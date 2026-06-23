mod artifacts;
mod clipboard;
mod directories;
mod formats;
mod paths;
mod recording;

pub(super) use artifacts::frame_capture_artifact;
pub(super) use clipboard::copy_image_file_to_clipboard;
pub(super) use directories::{
    capture_directory_for_app, normalize_capture_directory_override, recording_directory_for_app,
};
pub(super) use formats::{
    normalize_capture_image_format, normalize_recording_container_format,
    recording_container_format_for_method,
};
pub(super) use paths::{
    capture_file_stem, capture_output_path, current_time_ms_for_capture,
    plugin_frame_capture_output_path, recording_output_path,
};
#[cfg(test)]
pub(super) use recording::{
    ensure_recording_output_has_content, recording_dump_start_position,
    recording_output_has_content,
};
pub(super) use recording::{
    recording_method_for_media_path, recording_time_arg, stop_recording_for_player,
};

use super::{PreviewKind, ShellPreviewFormat, ShellPreviewRegistrationSummary};

#[cfg_attr(not(windows), allow(dead_code))]
pub(in crate::shell_preview) fn registration_summary(
    formats: &[ShellPreviewFormat],
) -> ShellPreviewRegistrationSummary {
    ShellPreviewRegistrationSummary {
        registered_count: formats.len(),
        video_count: formats
            .iter()
            .filter(|format| format.kind == PreviewKind::Video)
            .count(),
        audio_count: formats
            .iter()
            .filter(|format| format.kind == PreviewKind::Audio)
            .count(),
        extensions: formats
            .iter()
            .map(|format| format.extension.to_string())
            .collect(),
    }
}

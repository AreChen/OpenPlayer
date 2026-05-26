mod catalog;
mod selection;
mod summary;

#[cfg(test)]
mod tests;

use serde::Serialize;

pub(super) use catalog::PREVIEW_FORMATS;
pub(super) use selection::filter_preview_formats;
pub(super) use summary::registration_summary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PreviewKind {
    Video,
    Audio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ShellPreviewFormat {
    pub(super) extension: &'static str,
    pub(super) mime: &'static str,
    pub(super) kind: PreviewKind,
    pub(super) common: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellPreviewRegistrationSummary {
    registered_count: usize,
    video_count: usize,
    audio_count: usize,
    extensions: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellPreviewFormatInfo {
    extension: &'static str,
    mime: &'static str,
    kind: &'static str,
    common: bool,
}

impl ShellPreviewFormat {
    const fn video(extension: &'static str, mime: &'static str) -> Self {
        Self {
            extension,
            mime,
            kind: PreviewKind::Video,
            common: false,
        }
    }

    const fn video_common(extension: &'static str, mime: &'static str) -> Self {
        Self {
            extension,
            mime,
            kind: PreviewKind::Video,
            common: true,
        }
    }

    const fn audio(extension: &'static str, mime: &'static str) -> Self {
        Self {
            extension,
            mime,
            kind: PreviewKind::Audio,
            common: false,
        }
    }

    const fn audio_common(extension: &'static str, mime: &'static str) -> Self {
        Self {
            extension,
            mime,
            kind: PreviewKind::Audio,
            common: true,
        }
    }

    pub(super) fn info(self) -> ShellPreviewFormatInfo {
        ShellPreviewFormatInfo {
            extension: self.extension,
            mime: self.mime,
            kind: self.kind.as_str(),
            common: self.common,
        }
    }
}

impl PreviewKind {
    pub(super) fn perceived_type(self) -> &'static str {
        match self {
            PreviewKind::Video => "video",
            PreviewKind::Audio => "audio",
        }
    }

    fn as_str(self) -> &'static str {
        self.perceived_type()
    }
}

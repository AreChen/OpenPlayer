use std::{
    fs,
    path::{Path, PathBuf},
};

const SUPPORTED_SUBTITLE_EXTENSIONS: &[&str] = &["ass", "srt", "ssa", "sub", "vtt"];

pub(crate) fn is_supported_subtitle_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SUPPORTED_SUBTITLE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

pub(crate) fn discover_sidecar_subtitles(media_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = media_path.parent() else {
        return Vec::new();
    };
    let Some(media_stem) = media_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    let mut subtitles: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_supported_subtitle_path(path))
        .filter(|path| is_matching_sidecar_stem(path, media_stem))
        .filter(|path| path.is_file())
        .collect();
    subtitles.sort_by_cached_key(|path| {
        let exact = path.file_stem().is_some_and(|stem| stem == media_stem);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        (!exact, name.to_ascii_lowercase(), name.into_owned())
    });
    subtitles
}

fn is_matching_sidecar_stem(path: &Path, media_stem: &str) -> bool {
    let Some(candidate) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    candidate == media_stem
        || candidate
            .strip_prefix(media_stem)
            .is_some_and(|suffix| suffix.starts_with(['.', '-', '_']))
}

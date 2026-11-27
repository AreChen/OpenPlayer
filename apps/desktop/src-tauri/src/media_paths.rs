mod collect;
#[cfg(not(windows))]
pub(crate) mod commands;
mod extensions;

#[cfg(windows)]
pub(crate) use extensions::SUPPORTED_MEDIA_EXTENSIONS;
#[cfg(all(windows, feature = "windows-gpui"))]
pub(crate) use extensions::is_supported_media_path;
mod sort;
#[cfg(all(windows, feature = "windows-gpui"))]
pub(crate) use sort::natural_name_key;
#[cfg(any(not(windows), test))]
mod startup;
#[cfg(feature = "mpv-embed")]
mod streams;
#[cfg(feature = "mpv-embed")]
mod subtitles;

#[cfg(windows)]
pub(crate) use collect::collect_media_files_from_paths;
#[cfg(not(windows))]
pub use startup::StartupMediaState;
#[cfg(feature = "mpv-embed")]
pub(crate) use streams::is_hls_manifest_media_url;
#[cfg(feature = "mpv-embed")]
pub(crate) use subtitles::discover_sidecar_subtitles;
#[cfg(all(not(windows), feature = "mpv-embed"))]
pub(crate) use subtitles::is_supported_subtitle_path;

#[cfg(test)]
mod tests;

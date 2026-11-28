mod collect;
mod extensions;

#[cfg(windows)]
pub(crate) use extensions::SUPPORTED_MEDIA_EXTENSIONS;
pub(crate) use extensions::is_supported_media_path;
mod sort;
pub(crate) use sort::natural_name_key;
#[cfg(test)]
mod startup;
mod streams;
mod subtitles;

pub(crate) use collect::collect_media_files_from_paths;
pub(crate) use streams::is_hls_manifest_media_url;
pub(crate) use subtitles::discover_sidecar_subtitles;

#[cfg(test)]
mod tests;

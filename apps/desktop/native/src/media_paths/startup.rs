use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use super::{
    collect::collect_media_files_in_directory, extensions::is_supported_media_path,
    sort::sort_media_paths,
};

#[derive(Clone, Default)]
pub struct StartupMediaState {
    paths: Vec<String>,
}

impl StartupMediaState {
    pub fn from_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut paths = Vec::new();
        for arg in args.into_iter().skip(1) {
            let path = PathBuf::from(arg.into());
            if is_flag_like_path(&path) {
                continue;
            }
            if path.is_dir() {
                if let Ok(directory_paths) = collect_media_files_in_directory(&path) {
                    paths.extend(directory_paths.into_iter().map(PathBuf::from));
                }
            } else if path.is_file() && is_supported_media_path(&path) {
                paths.push(path);
            }
        }
        sort_media_paths(&mut paths);

        Self {
            paths: paths
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
        }
    }

    pub fn paths(&self) -> Vec<String> {
        self.paths.clone()
    }
}

fn is_flag_like_path(path: &Path) -> bool {
    path.as_os_str()
        .to_str()
        .map(|text| text.starts_with('-'))
        .unwrap_or(false)
}

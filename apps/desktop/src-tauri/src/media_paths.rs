use std::{
    cmp::Ordering,
    ffi::OsString,
    path::{Path, PathBuf},
};

const SUPPORTED_MEDIA_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "3gp2", "3gpp", "aac", "ac3", "adts", "aif", "aifc", "aiff", "alac", "amr",
    "ape", "asf", "au", "avi", "awb", "caf", "dff", "divx", "dsf", "dts", "dtshd", "dv", "dvr-ms",
    "eac3", "f4v", "flac", "flv", "gsm", "h264", "h265", "hevc", "m1v", "m2t", "m2ts", "m2v",
    "m4a", "m4b", "m4r", "m4v", "mk3d", "mka", "mkv", "mlp", "mov", "mp1", "mp2", "mp3", "mp4",
    "mp4v", "mpa", "mpc", "mpe", "mpeg", "mpg", "mpv", "mts", "mxf", "nsv", "nut", "oga", "ogg",
    "ogm", "ogv", "opus", "qt", "ra", "rm", "rmvb", "roq", "snd", "spx", "tak", "tod", "trp", "ts",
    "tta", "vob", "voc", "wav", "weba", "webm", "wm", "wma", "wmv", "wv", "y4m",
];

#[derive(Clone, Default)]
pub struct StartupMediaState {
    paths: Vec<String>,
}

impl StartupMediaState {
    pub fn from_args<I, S>(_args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut paths = Vec::new();
        for arg in _args.into_iter().skip(1) {
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

pub fn collect_media_files_in_directory(_directory: &Path) -> Result<Vec<String>, String> {
    if !_directory.is_dir() {
        return Err("selected path is not a directory".to_string());
    }

    let mut paths = Vec::new();
    let entries = std::fs::read_dir(_directory)
        .map_err(|error| format!("failed to read media directory: {error}"))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("failed to read media directory entry: {error}"))?;
        let path = entry.path();
        if path.is_file() && is_supported_media_path(&path) {
            paths.push(path);
        }
    }
    sort_media_paths(&mut paths);

    Ok(paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub fn media_files_in_directory(path: String) -> Result<Vec<String>, String> {
    collect_media_files_in_directory(Path::new(&path))
}

#[tauri::command]
pub fn startup_media_paths(state: tauri::State<'_, StartupMediaState>) -> Vec<String> {
    state.paths()
}

fn is_supported_media_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            SUPPORTED_MEDIA_EXTENSIONS
                .iter()
                .any(|supported| *supported == extension)
        })
        .unwrap_or(false)
}

fn is_flag_like_path(path: &Path) -> bool {
    path.as_os_str()
        .to_str()
        .map(|text| text.starts_with('-'))
        .unwrap_or(false)
}

fn sort_media_paths(paths: &mut [PathBuf]) {
    paths.sort_by(|left, right| {
        natural_path_cmp(left, right).then_with(|| {
            left.to_string_lossy()
                .to_ascii_lowercase()
                .cmp(&right.to_string_lossy().to_ascii_lowercase())
        })
    });
}

fn natural_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left_name = path_file_name(left);
    let right_name = path_file_name(right);
    natural_str_cmp(&left_name, &right_name)
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn natural_str_cmp(left: &str, right: &str) -> Ordering {
    let left_tokens = natural_tokens(left);
    let right_tokens = natural_tokens(right);
    for (left_token, right_token) in left_tokens.iter().zip(right_tokens.iter()) {
        let ordering = left_token.cmp(right_token);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left_tokens.len().cmp(&right_tokens.len())
}

#[derive(Debug, Eq, PartialEq)]
enum NaturalToken {
    Text(String),
    Number { value: String, raw_len: usize },
}

impl Ord for NaturalToken {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
            (
                Self::Number {
                    value: left,
                    raw_len: left_len,
                },
                Self::Number {
                    value: right,
                    raw_len: right_len,
                },
            ) => left
                .len()
                .cmp(&right.len())
                .then_with(|| left.cmp(right))
                .then_with(|| left_len.cmp(right_len)),
            (Self::Number { .. }, Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number { .. }) => Ordering::Greater,
        }
    }
}

impl PartialOrd for NaturalToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn natural_tokens(text: &str) -> Vec<NaturalToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for character in text.chars() {
        let is_digit = character.is_ascii_digit();
        if current_is_digit.is_some_and(|digit| digit != is_digit) {
            tokens.push(natural_token(&current, current_is_digit.unwrap_or(false)));
            current.clear();
        }
        current_is_digit = Some(is_digit);
        current.push(character);
    }

    if !current.is_empty() {
        tokens.push(natural_token(&current, current_is_digit.unwrap_or(false)));
    }

    tokens
}

fn natural_token(text: &str, is_digit: bool) -> NaturalToken {
    if is_digit {
        let trimmed = text.trim_start_matches('0');
        NaturalToken::Number {
            value: if trimmed.is_empty() {
                "0".to_string()
            } else {
                trimmed.to_string()
            },
            raw_len: text.len(),
        }
    } else {
        NaturalToken::Text(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_temp_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("temp directory should be created");
        directory
    }

    #[test]
    fn collects_supported_media_files_sorted_by_natural_filename() {
        let directory = create_temp_directory("media-sort");
        for name in [
            "episode 10.mkv",
            "episode 2.mp4",
            "episode 01.avi",
            "episode 03.mxf",
            "episode 04.wv",
            "poster.jpg",
            "episode 3.txt",
        ] {
            std::fs::write(directory.join(name), b"fixture").expect("fixture should be written");
        }
        std::fs::create_dir(directory.join("nested.mp4"))
            .expect("directory fixture should be created");

        let files =
            collect_media_files_in_directory(&directory).expect("media files should be read");
        let names: Vec<String> = files.iter().map(|path| media_file_name(path)).collect();

        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(
            names,
            vec![
                "episode 01.avi",
                "episode 2.mp4",
                "episode 03.mxf",
                "episode 04.wv",
                "episode 10.mkv"
            ]
        );
    }

    #[test]
    fn startup_media_paths_filter_flags_and_unsupported_extensions() {
        let directory = create_temp_directory("startup-paths");
        let media = directory.join("clip 2.mp4");
        let media_late = directory.join("clip 10.mkv");
        let note = directory.join("notes.txt");
        std::fs::write(&media_late, b"media").expect("media fixture should be written");
        std::fs::write(&note, b"note").expect("note fixture should be written");
        std::fs::write(&media, b"media").expect("media fixture should be written");

        let state = StartupMediaState::from_args([
            OsString::from("openplayer.exe"),
            OsString::from("--flag"),
            note.clone().into_os_string(),
            media_late.clone().into_os_string(),
            media.clone().into_os_string(),
        ]);

        let names: Vec<String> = state
            .paths()
            .iter()
            .map(|path| media_file_name(path))
            .collect();
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(names, vec!["clip 2.mp4", "clip 10.mkv"]);
    }

    fn media_file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .expect("path should have a UTF-8 file name")
            .to_string()
    }
}

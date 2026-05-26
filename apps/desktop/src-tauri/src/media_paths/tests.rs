use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use super::{
    collect::collect_media_files_from_paths, collect::collect_media_files_in_directory,
    startup::StartupMediaState,
};

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
    std::fs::create_dir(directory.join("nested.mp4")).expect("directory fixture should be created");

    let files = collect_media_files_in_directory(&directory).expect("media files should be read");
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

#[test]
fn collects_media_files_from_mixed_files_and_directories() {
    let directory = create_temp_directory("mixed-paths");
    let clip_10 = directory.join("clip 10.mkv");
    let clip_2 = directory.join("clip 2.mp4");
    let note = directory.join("notes.txt");
    std::fs::write(&clip_10, b"media").expect("media fixture should be written");
    std::fs::write(&clip_2, b"media").expect("media fixture should be written");
    std::fs::write(&note, b"note").expect("note fixture should be written");

    let files = collect_media_files_from_paths(&[
        directory.to_string_lossy().to_string(),
        clip_10.to_string_lossy().to_string(),
        note.to_string_lossy().to_string(),
    ])
    .expect("mixed paths should be collected");
    let names: Vec<String> = files.iter().map(|path| media_file_name(path)).collect();
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

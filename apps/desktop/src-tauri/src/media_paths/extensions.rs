use std::path::Path;

const SUPPORTED_MEDIA_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "3gp2", "3gpp", "aac", "ac3", "adts", "aif", "aifc", "aiff", "alac", "amr",
    "ape", "asf", "au", "avi", "awb", "caf", "dff", "divx", "dsf", "dts", "dtshd", "dv", "dvr-ms",
    "eac3", "f4v", "flac", "flv", "gsm", "h264", "h265", "hevc", "m1v", "m2t", "m2ts", "m2v",
    "m4a", "m4b", "m4r", "m4v", "mk3d", "mka", "mkv", "mlp", "mov", "mp1", "mp2", "mp3", "mp4",
    "mp4v", "mpa", "mpc", "mpe", "mpeg", "mpg", "mpv", "mts", "mxf", "nsv", "nut", "oga", "ogg",
    "ogm", "ogv", "opus", "qt", "ra", "rm", "rmvb", "roq", "snd", "spx", "tak", "tod", "trp", "ts",
    "tta", "vob", "voc", "wav", "weba", "webm", "wm", "wma", "wmv", "wv", "y4m",
];

pub(in crate::media_paths) fn is_supported_media_path(path: &Path) -> bool {
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

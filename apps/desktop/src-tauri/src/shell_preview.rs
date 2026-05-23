use serde::Serialize;
use std::collections::HashSet;

#[cfg(windows)]
const THUMBNAIL_HANDLER_GUID: &str = "{e357fccd-a995-4576-b01f-234630154e96}";
#[cfg(windows)]
const LEGACY_IMAGE_HANDLER_GUID: &str = "{BB2E617C-0920-11D1-9A0B-00C04FC2D6C1}";
#[cfg(windows)]
const PROPERTY_THUMBNAIL_HANDLER_CLSID: &str = "{9DBD2C50-62AD-11D0-B806-00C04FD706EC}";
#[cfg(windows)]
const OPENPLAYER_PROG_ID: &str = "OpenPlayer.Media";
#[cfg(windows)]
const OPENPLAYER_REGISTERED_APP_NAME: &str = "OpenPlayer";
#[cfg(windows)]
const OPENPLAYER_CAPABILITIES_KEY: &str = "Software\\OpenPlayer\\Capabilities";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreviewKind {
    Video,
    Audio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShellPreviewFormat {
    extension: &'static str,
    mime: &'static str,
    kind: PreviewKind,
    common: bool,
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

const PREVIEW_FORMATS: &[ShellPreviewFormat] = &[
    ShellPreviewFormat::video("3g2", "video/3gpp2"),
    ShellPreviewFormat::video_common("3gp", "video/3gpp"),
    ShellPreviewFormat::video("3gp2", "video/3gpp2"),
    ShellPreviewFormat::video("3gpp", "video/3gpp"),
    ShellPreviewFormat::video("asf", "video/x-ms-asf"),
    ShellPreviewFormat::video_common("avi", "video/avi"),
    ShellPreviewFormat::video("divx", "video/divx"),
    ShellPreviewFormat::video("dv", "video/dv"),
    ShellPreviewFormat::video("dvr-ms", "video/x-ms-dvr"),
    ShellPreviewFormat::video("f4v", "video/mp4"),
    ShellPreviewFormat::video_common("flv", "video/x-flv"),
    ShellPreviewFormat::video("h264", "video/h264"),
    ShellPreviewFormat::video("h265", "video/h265"),
    ShellPreviewFormat::video("hevc", "video/h265"),
    ShellPreviewFormat::video("m1v", "video/mpeg"),
    ShellPreviewFormat::video("m2t", "video/mp2t"),
    ShellPreviewFormat::video_common("m2ts", "video/vnd.dlna.mpeg-tts"),
    ShellPreviewFormat::video("m2v", "video/mpeg"),
    ShellPreviewFormat::video_common("m4v", "video/mp4"),
    ShellPreviewFormat::video("mk3d", "video/x-matroska"),
    ShellPreviewFormat::video_common("mkv", "video/x-matroska"),
    ShellPreviewFormat::video_common("mov", "video/quicktime"),
    ShellPreviewFormat::video_common("mp4", "video/mp4"),
    ShellPreviewFormat::video("mp4v", "video/mp4"),
    ShellPreviewFormat::video("mpe", "video/mpeg"),
    ShellPreviewFormat::video_common("mpeg", "video/mpeg"),
    ShellPreviewFormat::video_common("mpg", "video/mpeg"),
    ShellPreviewFormat::video("mpv", "video/mpeg"),
    ShellPreviewFormat::video("mts", "video/mp2t"),
    ShellPreviewFormat::video("mxf", "application/mxf"),
    ShellPreviewFormat::video("nsv", "video/x-nsv"),
    ShellPreviewFormat::video("nut", "video/x-nut"),
    ShellPreviewFormat::video("ogm", "video/ogg"),
    ShellPreviewFormat::video_common("ogv", "video/ogg"),
    ShellPreviewFormat::video("qt", "video/quicktime"),
    ShellPreviewFormat::video("rm", "application/vnd.rn-realmedia"),
    ShellPreviewFormat::video("rmvb", "application/vnd.rn-realmedia-vbr"),
    ShellPreviewFormat::video("roq", "video/x-roq"),
    ShellPreviewFormat::video("tod", "video/mp2t"),
    ShellPreviewFormat::video("trp", "video/mp2t"),
    ShellPreviewFormat::video("ts", "video/mp2t"),
    ShellPreviewFormat::video("vob", "video/dvd"),
    ShellPreviewFormat::video_common("webm", "video/webm"),
    ShellPreviewFormat::video("wm", "video/x-ms-wm"),
    ShellPreviewFormat::video_common("wmv", "video/x-ms-wmv"),
    ShellPreviewFormat::video("y4m", "video/x-yuv4mpeg"),
    ShellPreviewFormat::audio_common("aac", "audio/aac"),
    ShellPreviewFormat::audio("ac3", "audio/ac3"),
    ShellPreviewFormat::audio("adts", "audio/aac"),
    ShellPreviewFormat::audio("aif", "audio/aiff"),
    ShellPreviewFormat::audio("aifc", "audio/aiff"),
    ShellPreviewFormat::audio("aiff", "audio/aiff"),
    ShellPreviewFormat::audio("alac", "audio/mp4"),
    ShellPreviewFormat::audio("amr", "audio/amr"),
    ShellPreviewFormat::audio("ape", "audio/x-ape"),
    ShellPreviewFormat::audio("au", "audio/basic"),
    ShellPreviewFormat::audio("awb", "audio/amr-wb"),
    ShellPreviewFormat::audio("caf", "audio/x-caf"),
    ShellPreviewFormat::audio("dff", "audio/x-dff"),
    ShellPreviewFormat::audio("dsf", "audio/x-dsf"),
    ShellPreviewFormat::audio("dts", "audio/vnd.dts"),
    ShellPreviewFormat::audio("dtshd", "audio/vnd.dts.hd"),
    ShellPreviewFormat::audio("eac3", "audio/eac3"),
    ShellPreviewFormat::audio_common("flac", "audio/flac"),
    ShellPreviewFormat::audio("gsm", "audio/x-gsm"),
    ShellPreviewFormat::audio_common("m4a", "audio/mp4"),
    ShellPreviewFormat::audio_common("m4b", "audio/mp4"),
    ShellPreviewFormat::audio("m4r", "audio/mp4"),
    ShellPreviewFormat::audio("mka", "audio/x-matroska"),
    ShellPreviewFormat::audio("mlp", "audio/true-hd"),
    ShellPreviewFormat::audio("mp1", "audio/mpeg"),
    ShellPreviewFormat::audio("mp2", "audio/mpeg"),
    ShellPreviewFormat::audio_common("mp3", "audio/mpeg"),
    ShellPreviewFormat::audio("mpa", "audio/mpeg"),
    ShellPreviewFormat::audio("mpc", "audio/x-musepack"),
    ShellPreviewFormat::audio_common("oga", "audio/ogg"),
    ShellPreviewFormat::audio_common("ogg", "audio/ogg"),
    ShellPreviewFormat::audio_common("opus", "audio/ogg"),
    ShellPreviewFormat::audio("ra", "audio/vnd.rn-realaudio"),
    ShellPreviewFormat::audio("snd", "audio/basic"),
    ShellPreviewFormat::audio("spx", "audio/ogg"),
    ShellPreviewFormat::audio("tak", "audio/x-tak"),
    ShellPreviewFormat::audio("tta", "audio/x-tta"),
    ShellPreviewFormat::audio("voc", "audio/x-voc"),
    ShellPreviewFormat::audio_common("wav", "audio/wav"),
    ShellPreviewFormat::audio("weba", "audio/webm"),
    ShellPreviewFormat::audio("wma", "audio/x-ms-wma"),
    ShellPreviewFormat::audio("wv", "audio/x-wavpack"),
];

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

    fn info(self) -> ShellPreviewFormatInfo {
        ShellPreviewFormatInfo {
            extension: self.extension,
            mime: self.mime,
            kind: self.kind.as_str(),
            common: self.common,
        }
    }
}

impl PreviewKind {
    fn perceived_type(self) -> &'static str {
        match self {
            PreviewKind::Video => "video",
            PreviewKind::Audio => "audio",
        }
    }

    fn as_str(self) -> &'static str {
        self.perceived_type()
    }
}

#[tauri::command]
pub fn shell_preview_formats() -> Vec<ShellPreviewFormatInfo> {
    PREVIEW_FORMATS
        .iter()
        .copied()
        .map(ShellPreviewFormat::info)
        .collect()
}

#[tauri::command]
pub fn shell_preview_open_default_apps_settings() -> Result<(), String> {
    open_default_apps_settings()
}

#[tauri::command]
pub fn shell_preview_register_formats(
    app: tauri::AppHandle,
    selected_extensions: Vec<String>,
) -> Result<ShellPreviewRegistrationSummary, String> {
    let formats = filter_preview_formats(&selected_extensions)?;
    register_shell_previews(&app, &formats)
}

fn filter_preview_formats(
    selected_extensions: &[String],
) -> Result<Vec<ShellPreviewFormat>, String> {
    let selected: HashSet<String> = selected_extensions
        .iter()
        .map(|extension| normalize_extension(extension))
        .filter(|extension| !extension.is_empty())
        .collect();

    if selected.is_empty() {
        return Err("select at least one preview format".to_string());
    }

    let formats: Vec<ShellPreviewFormat> = PREVIEW_FORMATS
        .iter()
        .copied()
        .filter(|format| selected.contains(format.extension))
        .collect();

    if formats.len() != selected.len() {
        let known: HashSet<&str> = formats.iter().map(|format| format.extension).collect();
        let mut unknown: Vec<String> = selected
            .into_iter()
            .filter(|extension| !known.contains(extension.as_str()))
            .collect();
        unknown.sort();
        return Err(format!(
            "unsupported preview format: {}",
            unknown.join(", ")
        ));
    }

    Ok(formats)
}

fn normalize_extension(extension: &str) -> String {
    extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}

#[cfg_attr(not(windows), allow(dead_code))]
fn registration_summary(formats: &[ShellPreviewFormat]) -> ShellPreviewRegistrationSummary {
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

#[cfg(windows)]
fn register_shell_previews(
    app: &tauri::AppHandle,
    formats: &[ShellPreviewFormat],
) -> Result<ShellPreviewRegistrationSummary, String> {
    let handler = openplayer_handler_registration(app)?;
    register_openplayer_handlers(formats, &handler)?;

    write_reg_dword(
        "Software\\Classes\\SystemFileAssociations\\video",
        "Treatment",
        3,
    )?;
    write_reg_dword(
        "Software\\Classes\\SystemFileAssociations\\video",
        "ThumbnailCutoff",
        1,
    )?;

    for format in formats {
        let extension_key = format!("Software\\Classes\\.{}", format.extension);
        write_reg_string(
            &extension_key,
            "PerceivedType",
            format.kind.perceived_type(),
        )?;
        write_reg_string(&extension_key, "Content Type", format.mime)?;
        write_reg_string(
            &format!("{extension_key}\\ShellEx\\{THUMBNAIL_HANDLER_GUID}"),
            "",
            PROPERTY_THUMBNAIL_HANDLER_CLSID,
        )?;
        write_reg_string(
            &format!("{extension_key}\\ShellEx\\{LEGACY_IMAGE_HANDLER_GUID}"),
            "",
            PROPERTY_THUMBNAIL_HANDLER_CLSID,
        )?;
        write_reg_none(
            &format!("{extension_key}\\OpenWithProgids"),
            OPENPLAYER_PROG_ID,
        )?;
        write_reg_string(
            &format!("{extension_key}\\OpenWithList\\{}", handler.executable_name),
            "",
            "",
        )?;
        write_reg_string(
            &format!("{OPENPLAYER_CAPABILITIES_KEY}\\FileAssociations"),
            &format!(".{}", format.extension),
            OPENPLAYER_PROG_ID,
        )?;
    }

    notify_shell_association_changed();
    Ok(registration_summary(formats))
}

#[cfg(not(windows))]
fn register_shell_previews(
    _app: &tauri::AppHandle,
    _formats: &[ShellPreviewFormat],
) -> Result<ShellPreviewRegistrationSummary, String> {
    Err("Explorer preview registration is only available on Windows".to_string())
}

#[cfg(windows)]
fn open_default_apps_settings() -> Result<(), String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = wide_null("open");
    let target = wide_null("ms-settings:defaultapps");
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if result <= 32 {
        Err(format!(
            "failed to open Windows default apps settings: code {result}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn open_default_apps_settings() -> Result<(), String> {
    Err("Windows default apps settings are only available on Windows".to_string())
}

#[cfg(windows)]
struct OpenPlayerHandlerRegistration {
    application_key: String,
    executable_name: String,
    icon: String,
    command: String,
}

#[cfg(windows)]
fn openplayer_handler_registration(
    _app: &tauri::AppHandle,
) -> Result<OpenPlayerHandlerRegistration, String> {
    let executable_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve OpenPlayer executable: {error}"))?;
    let executable_name = executable_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "failed to resolve OpenPlayer executable name".to_string())?;
    let executable = executable_path.to_string_lossy().to_string();
    let quoted_executable = format!("\"{executable}\"");

    Ok(OpenPlayerHandlerRegistration {
        application_key: format!("Software\\Classes\\Applications\\{executable_name}"),
        executable_name: executable_name.to_string(),
        icon: format!("{executable},0"),
        command: format!("{quoted_executable} \"%1\""),
    })
}

#[cfg(windows)]
fn register_openplayer_handlers(
    formats: &[ShellPreviewFormat],
    handler: &OpenPlayerHandlerRegistration,
) -> Result<(), String> {
    write_reg_string(
        "Software\\Classes\\OpenPlayer.Media",
        "",
        "OpenPlayer media file",
    )?;
    write_reg_string(
        "Software\\Classes\\OpenPlayer.Media\\DefaultIcon",
        "",
        &handler.icon,
    )?;
    write_reg_string("Software\\Classes\\OpenPlayer.Media\\shell", "", "open")?;
    write_reg_string(
        "Software\\Classes\\OpenPlayer.Media\\shell\\open",
        "",
        "Open with OpenPlayer",
    )?;
    write_reg_string(
        "Software\\Classes\\OpenPlayer.Media\\shell\\open\\command",
        "",
        &handler.command,
    )?;
    write_reg_string(
        &handler.application_key,
        "FriendlyAppName",
        OPENPLAYER_REGISTERED_APP_NAME,
    )?;
    write_reg_string(&handler.application_key, "ApplicationIcon", &handler.icon)?;
    write_reg_string(
        &format!("{}\\shell\\open\\command", handler.application_key),
        "",
        &handler.command,
    )?;
    write_reg_string(
        OPENPLAYER_CAPABILITIES_KEY,
        "ApplicationName",
        OPENPLAYER_REGISTERED_APP_NAME,
    )?;
    write_reg_string(
        OPENPLAYER_CAPABILITIES_KEY,
        "ApplicationDescription",
        "High-performance media player",
    )?;
    write_reg_string(
        OPENPLAYER_CAPABILITIES_KEY,
        "ApplicationIcon",
        &handler.icon,
    )?;
    write_reg_string(
        "Software\\RegisteredApplications",
        OPENPLAYER_REGISTERED_APP_NAME,
        OPENPLAYER_CAPABILITIES_KEY,
    )?;

    for format in formats {
        write_reg_string(
            &format!("{}\\SupportedTypes", handler.application_key),
            &format!(".{}", format.extension),
            "",
        )?;
    }

    Ok(())
}

#[cfg(windows)]
fn write_reg_string(subkey: &str, value_name: &str, value: &str) -> Result<(), String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegSetValueExW,
    };

    let mut key: HKEY = null_mut();
    let subkey_w = wide_null(subkey);
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr(), &mut key) };
    if status != 0 {
        return Err(format!(
            "failed to open registry key {subkey}: code {status}"
        ));
    }

    let name_w = wide_null(value_name);
    let value_name_ptr = if value_name.is_empty() {
        null()
    } else {
        name_w.as_ptr()
    };
    let value_w = wide_null(value);
    let status = unsafe {
        RegSetValueExW(
            key,
            value_name_ptr,
            0,
            REG_SZ,
            value_w.as_ptr().cast::<u8>(),
            (value_w.len() * 2) as u32,
        )
    };
    unsafe {
        RegCloseKey(key);
    }

    if status != 0 {
        return Err(format!(
            "failed to write registry value {subkey}\\{value_name}: code {status}"
        ));
    }

    Ok(())
}

#[cfg(windows)]
fn write_reg_none(subkey: &str, value_name: &str) -> Result<(), String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, REG_NONE, RegCloseKey, RegCreateKeyW, RegSetValueExW,
    };

    let mut key: HKEY = null_mut();
    let subkey_w = wide_null(subkey);
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr(), &mut key) };
    if status != 0 {
        return Err(format!(
            "failed to open registry key {subkey}: code {status}"
        ));
    }

    let name_w = wide_null(value_name);
    let status = unsafe { RegSetValueExW(key, name_w.as_ptr(), 0, REG_NONE, null(), 0) };
    unsafe {
        RegCloseKey(key);
    }

    if status != 0 {
        return Err(format!(
            "failed to write registry value {subkey}\\{value_name}: code {status}"
        ));
    }

    Ok(())
}

#[cfg(windows)]
fn write_reg_dword(subkey: &str, value_name: &str, value: u32) -> Result<(), String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, REG_DWORD, RegCloseKey, RegCreateKeyW, RegSetValueExW,
    };

    let mut key: HKEY = null_mut();
    let subkey_w = wide_null(subkey);
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr(), &mut key) };
    if status != 0 {
        return Err(format!(
            "failed to open registry key {subkey}: code {status}"
        ));
    }

    let name_w = wide_null(value_name);
    let value_bytes = value.to_le_bytes();
    let status = unsafe {
        RegSetValueExW(
            key,
            name_w.as_ptr(),
            0,
            REG_DWORD,
            value_bytes.as_ptr(),
            value_bytes.len() as u32,
        )
    };
    unsafe {
        RegCloseKey(key);
    }

    if status != 0 {
        return Err(format!(
            "failed to write registry value {subkey}\\{value_name}: code {status}"
        ));
    }

    Ok(())
}

#[cfg(windows)]
fn notify_shell_association_changed() {
    use std::ptr::null;
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_FLUSH, SHChangeNotify};

    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_FLUSH, null(), null());
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn preview_format_catalog_has_no_duplicate_extensions() {
        let mut extensions = HashSet::new();
        for format in PREVIEW_FORMATS {
            assert!(
                extensions.insert(format.extension),
                "duplicate preview extension: {}",
                format.extension
            );
        }
    }

    #[test]
    fn preview_format_catalog_covers_common_mpv_media_containers() {
        let extensions: HashSet<&str> = PREVIEW_FORMATS
            .iter()
            .map(|format| format.extension)
            .collect();

        for extension in [
            "mp4", "mkv", "avi", "webm", "mov", "flv", "m2ts", "vob", "mxf", "mp3", "flac", "m4a",
            "m4b", "amr", "caf", "spx", "opus", "wav", "wma", "wv",
        ] {
            assert!(
                extensions.contains(extension),
                "missing preview extension: {extension}"
            );
        }
    }

    #[test]
    fn registration_summary_counts_video_and_audio_formats() {
        let summary = registration_summary(PREVIEW_FORMATS);

        assert_eq!(summary.registered_count, PREVIEW_FORMATS.len());
        assert!(summary.video_count > 40);
        assert!(summary.audio_count > 30);
        assert_eq!(
            summary.registered_count,
            summary.video_count + summary.audio_count
        );
    }

    #[test]
    fn preview_format_catalog_marks_common_defaults_as_subset() {
        let formats = shell_preview_formats();
        let common: Vec<&ShellPreviewFormatInfo> =
            formats.iter().filter(|format| format.common).collect();

        assert!(common.len() > 10);
        assert!(common.len() < formats.len());
        assert!(common.iter().any(|format| format.extension == "mp4"));
        assert!(common.iter().any(|format| format.extension == "mkv"));
        assert!(common.iter().any(|format| format.extension == "mp3"));
        assert!(common.iter().any(|format| format.extension == "wav"));
        assert!(common.iter().any(|format| format.extension == "m4b"));
        assert!(!common.iter().any(|format| format.extension == "mxf"));
        assert!(!common.iter().any(|format| format.extension == "wv"));
    }

    #[test]
    fn selected_preview_registration_filters_to_requested_formats() {
        let formats =
            filter_preview_formats(&[".MP4".to_string(), "mkv".to_string(), "wv".to_string()])
                .expect("selected formats should be accepted");
        let extensions: Vec<&str> = formats.iter().map(|format| format.extension).collect();

        assert_eq!(extensions, vec!["mkv", "mp4", "wv"]);
    }

    #[test]
    fn selected_preview_registration_rejects_empty_or_unknown_formats() {
        assert!(filter_preview_formats(&[]).is_err());
        assert!(filter_preview_formats(&["unknown".to_string()]).is_err());
    }
}

use crate::shell_preview::formats::{
    ShellPreviewFormat, ShellPreviewRegistrationSummary, registration_summary,
};

use super::registry::{
    notify_shell_association_changed, write_reg_dword, write_reg_none, write_reg_string,
};

const THUMBNAIL_HANDLER_GUID: &str = "{e357fccd-a995-4576-b01f-234630154e96}";
const LEGACY_IMAGE_HANDLER_GUID: &str = "{BB2E617C-0920-11D1-9A0B-00C04FC2D6C1}";
const PROPERTY_THUMBNAIL_HANDLER_CLSID: &str = "{9DBD2C50-62AD-11D0-B806-00C04FD706EC}";
const OPENPLAYER_PROG_ID: &str = "OpenPlayer.Media";
const OPENPLAYER_REGISTERED_APP_NAME: &str = "OpenPlayer";
const OPENPLAYER_CAPABILITIES_KEY: &str = "Software\\OpenPlayer\\Capabilities";

pub(in crate::shell_preview) fn register_shell_previews(
    app: &tauri::AppHandle,
    formats: &[ShellPreviewFormat],
) -> Result<ShellPreviewRegistrationSummary, String> {
    let handler = openplayer_handler_registration(app)?;
    register_openplayer_handlers(formats, &handler)?;
    register_system_video_thumbnail_defaults()?;

    for format in formats {
        register_format(format, &handler)?;
    }

    notify_shell_association_changed();
    Ok(registration_summary(formats))
}

struct OpenPlayerHandlerRegistration {
    application_key: String,
    executable_name: String,
    icon: String,
    command: String,
}

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

fn register_system_video_thumbnail_defaults() -> Result<(), String> {
    write_reg_dword(
        "Software\\Classes\\SystemFileAssociations\\video",
        "Treatment",
        3,
    )?;
    write_reg_dword(
        "Software\\Classes\\SystemFileAssociations\\video",
        "ThumbnailCutoff",
        1,
    )
}

fn register_format(
    format: &ShellPreviewFormat,
    handler: &OpenPlayerHandlerRegistration,
) -> Result<(), String> {
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
    )
}

use std::process::Command;
#[tauri::command]
pub(crate) fn app_open_url(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !is_safe_external_url(trimmed) {
        return Err("invalid external url".to_string());
    }

    open_external_url(trimmed)
}

fn is_safe_external_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !url.chars().any(char::is_whitespace)
}

#[cfg(windows)]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open url: {error}"))
}

#[cfg(target_os = "macos")]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open url: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open url: {error}"))
}

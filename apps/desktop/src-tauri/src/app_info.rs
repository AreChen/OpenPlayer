#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppVersionInfo {
    name: &'static str,
    version: &'static str,
    license: &'static str,
    repository: &'static str,
    releases_url: &'static str,
}

const OPENPLAYER_REPOSITORY_URL: &str = "https://github.com/AreChen/OpenPlayer";
const OPENPLAYER_RELEASES_URL: &str = "https://github.com/AreChen/OpenPlayer/releases/latest";

#[tauri::command]
pub(crate) fn app_version() -> AppVersionInfo {
    AppVersionInfo {
        name: "OpenPlayer",
        version: env!("CARGO_PKG_VERSION"),
        license: env!("CARGO_PKG_LICENSE"),
        repository: OPENPLAYER_REPOSITORY_URL,
        releases_url: OPENPLAYER_RELEASES_URL,
    }
}

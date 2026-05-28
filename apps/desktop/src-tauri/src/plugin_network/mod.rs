mod execution;
mod types;
mod validation;

use tauri::Manager;
use types::{PluginNetworkRequestArgs, PluginNetworkResponse};

#[tauri::command]
pub(crate) async fn plugin_network_request(
    app: tauri::AppHandle,
    plugin_id: String,
    args: PluginNetworkRequestArgs,
) -> Result<PluginNetworkResponse, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
    execution::execute_plugin_network_request(app_data_dir, plugin_id, args).await
}

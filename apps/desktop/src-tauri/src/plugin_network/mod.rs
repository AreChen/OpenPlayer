mod execution;
mod types;
mod validation;

use types::{PluginNetworkRequestArgs, PluginNetworkResponse};

#[tauri::command]
pub(crate) async fn plugin_network_request(
    args: PluginNetworkRequestArgs,
) -> Result<PluginNetworkResponse, String> {
    execution::execute_plugin_network_request(args).await
}

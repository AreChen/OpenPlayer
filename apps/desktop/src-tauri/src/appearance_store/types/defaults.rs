use serde_json::Value;

use crate::appearance_store::SUPPORTED_PLUGIN_API_VERSION;

pub(in crate::appearance_store) fn default_plugin_action_args() -> Value {
    serde_json::json!({})
}

pub(in crate::appearance_store) fn default_plugin_api_version() -> String {
    SUPPORTED_PLUGIN_API_VERSION.to_string()
}

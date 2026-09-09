use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub(crate) async fn plugin_native_video_status(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
) -> Result<Value, String> {
    let modules = app
        .state::<crate::appearance_store::AppearanceStoreState>()
        .native_modules(&plugin_id)?;
    if !modules
        .iter()
        .any(|m| m.id == module_id && m.video_adapter.is_some())
    {
        return Err("module does not declare a native video adapter".into());
    }
    Ok(json!({"supported":false,"running":false,"attached":false,"filterEnabled":false}))
}

#[tauri::command]
pub(crate) async fn plugin_native_video_attach(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
    options: Value,
) -> Result<Value, String> {
    let _ = (app, plugin_id, module_id, options);
    Err("native video attachment requires Windows x64 and mpv-embed".into())
}

#[tauri::command]
pub(crate) async fn plugin_native_video_detach(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
) -> Result<Value, String> {
    let _ = (app, plugin_id, module_id);
    Err("native video attachment requires Windows x64 and mpv-embed".into())
}

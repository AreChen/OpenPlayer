use super::{NativeModuleInfo, REGISTRY, Session, current_target};
use crate::appearance_store::AppearanceStoreState;
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[tauri::command]
pub(crate) async fn plugin_native_validate_video_plan(
    app: AppHandle,
    plugin_id: String,
    plan: super::video_plan::VideoPlan,
) -> Result<super::video_plan::ValidatedVideoPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let modules = app
            .state::<AppearanceStoreState>()
            .native_modules(&plugin_id)?;
        super::video_plan::validate_plan(plan, &modules)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn plugin_native_list(
    app: AppHandle,
    plugin_id: String,
) -> Result<Vec<NativeModuleInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let modules = app
            .state::<AppearanceStoreState>()
            .native_modules(&plugin_id)?;
        let registry = REGISTRY.lock().map_err(|_| "native registry unavailable")?;
        Ok(modules
            .into_iter()
            .map(|m| NativeModuleInfo {
                supported: m.targets.contains_key(&current_target()),
                running: registry
                    .sessions
                    .get(&(plugin_id.clone(), m.id.clone()))
                    .is_some_and(|s| s.running()),
                id: m.id,
                protocol: m.protocol,
                methods: m.methods,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn plugin_native_start(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
) -> Result<(), String> {
    static CONSENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _consent = CONSENT
        .try_lock()
        .map_err(|_| "another native launch is awaiting confirmation")?;
    let generation = REGISTRY
        .lock()
        .map_err(|_| "native registry unavailable")?
        .generation;
    let handle = app.clone();
    let launch = tauri::async_runtime::spawn_blocking(move || {
        handle
            .state::<AppearanceStoreState>()
            .native_launch(&plugin_id, &module_id)
    })
    .await
    .map_err(|e| e.to_string())??;
    let key = (launch.plugin_id.clone(), launch.module.id.clone());
    let existing_key = key.clone();
    let exists = tauri::async_runtime::spawn_blocking(move || {
        let mut registry = REGISTRY.lock().map_err(|_| "native registry unavailable")?;
        registry.prune_exited()?;
        if registry.sessions.contains_key(&existing_key) {
            return Ok(true);
        }
        if registry.sessions.len() >= 16 {
            return Err("native session limit reached".into());
        }
        Ok::<_, String>(false)
    })
    .await
    .map_err(|e| e.to_string())??;
    if exists {
        return Ok(());
    }
    let (title, message) = super::consent::message(&launch);
    let allowed = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .message(message)
            .title(title)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancel)
            .blocking_show()
    })
    .await
    .map_err(|e| e.to_string())?;
    if !allowed {
        return Err("native launch was declined".into());
    }
    let session_key = key.clone();
    let session = tauri::async_runtime::spawn_blocking(move || {
        let target = launch
            .module
            .targets
            .get(&current_target())
            .ok_or("unsupported native target")?;
        super::verify_executable(&launch.executable, &target.sha256)?;
        let mut registry = REGISTRY.lock().map_err(|_| "native registry unavailable")?;
        if registry.generation != generation {
            return Err("plugins changed while awaiting confirmation; retry launch".into());
        }
        let session = Session::spawn(launch)?;
        registry.sessions.insert(session_key, session.clone());
        Ok::<_, String>(session)
    })
    .await
    .map_err(|e| e.to_string())??;
    if let Err(error) = session.initialize().await {
        session.tree.stop();
        tauri::async_runtime::spawn_blocking(move || {
            session.stop_and_wait()?;
            let mut registry = REGISTRY.lock().map_err(|_| "native registry unavailable")?;
            if registry
                .sessions
                .get(&key)
                .is_some_and(|s| std::sync::Arc::ptr_eq(s, &session))
            {
                registry.sessions.remove(&key);
            }
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| e.to_string())??;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn plugin_native_call(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
    method: String,
    params: Value,
    timeout_ms: Option<u64>,
) -> Result<Value, String> {
    if method.starts_with("host.") {
        return Err("host protocol methods are reserved".into());
    }
    let timeout = timeout_ms.unwrap_or(5000);
    if !(1..=5000).contains(&timeout) {
        return Err("native timeout must be 1..5000 milliseconds".into());
    }
    let id = plugin_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppearanceStoreState>().native_modules(&id)
    })
    .await
    .map_err(|e| e.to_string())??;
    let session = REGISTRY
        .lock()
        .map_err(|_| "native registry unavailable")?
        .sessions
        .get(&(plugin_id, module_id))
        .cloned()
        .ok_or("native module is not started")?;
    session.request(&method, params, timeout).await
}

#[tauri::command]
pub(crate) async fn plugin_native_stop(
    plugin_id: String,
    module_id: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        REGISTRY
            .lock()
            .map_err(|_| "native registry unavailable")?
            .stop_matching(|(id, module)| {
                id == &plugin_id && module_id.as_ref().is_none_or(|m| m == module)
            })
    })
    .await
    .map_err(|e| e.to_string())?
}

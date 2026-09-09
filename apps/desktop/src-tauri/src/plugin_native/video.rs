//! Single-stage CPU frame attachment, deliberately separate from plan validation.
use super::{REGISTRY, Session};
use crate::{appearance_store::AppearanceStoreState, mpv_embed::native_video_filter as filter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

static MEDIA: Mutex<()> = Mutex::new(());

// Order: media -> registry -> attachment -> mpv. Media replacement retains this
// guard through player creation, so no attachment can bind to the outgoing player.
pub(crate) fn media_change_guard() -> Result<MutexGuard<'static, ()>, String> {
    let guard = MEDIA
        .lock()
        .map_err(|_| "native video operation unavailable")?;
    let mut registry = REGISTRY.lock().map_err(|_| "native registry unavailable")?;
    let keys: Vec<_> = registry
        .sessions
        .iter()
        .filter(|(_, session)| session.launch.module.video_adapter.is_some())
        .map(|(key, _)| key.clone())
        .collect();
    if !keys.is_empty() {
        registry.stop_matching(|key| keys.contains(key))?;
    }
    Ok(guard)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    port: u16,
    token: String,
}

fn endpoint(value: Value) -> Result<Value, String> {
    let endpoint: Endpoint = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if endpoint.port == 0
        || endpoint.token.len() != 64
        || !endpoint.token.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("invalid native frame endpoint".into());
    }
    serde_json::to_value(endpoint).map_err(|e| e.to_string())
}

fn frame_options(mut options: Value) -> Result<(Value, bool), String> {
    let conversion = match &mut options {
        Value::Object(object) => object.remove("inputConversion"),
        Value::Null => None,
        _ => return Err("native video options must be an object".into()),
    };
    let normalize = match conversion.as_ref().and_then(Value::as_str) {
        None if conversion.is_none() => false,
        Some("none") => false,
        Some("sdr-bt709") => true,
        _ => return Err("inputConversion must be none or sdr-bt709".into()),
    };
    Ok((options, normalize))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VideoStatus {
    supported: bool,
    running: bool,
    attached: bool,
    filter_enabled: bool,
}

fn session(app: &AppHandle, plugin: &str, module: &str) -> Result<Option<Arc<Session>>, String> {
    let modules = app.state::<AppearanceStoreState>().native_modules(plugin)?;
    if !modules
        .iter()
        .any(|m| m.id == module && m.video_adapter.as_deref() == Some("vapoursynth-rgb-v1"))
    {
        return Err("module does not declare a native video adapter".into());
    }
    Ok(REGISTRY
        .lock()
        .map_err(|_| "native registry unavailable")?
        .sessions
        .get(&(plugin.into(), module.into()))
        .cloned())
}

fn status(app: &AppHandle, session: Option<&Arc<Session>>) -> Result<VideoStatus, String> {
    let running = session.is_some_and(|s| s.running());
    let attached = match session {
        Some(s) => s
            .attachment
            .lock()
            .map_err(|_| "native attachment unavailable")?
            .attached(),
        None => false,
    };
    Ok(VideoStatus {
        supported: true,
        running,
        attached,
        filter_enabled: attached && filter::enabled(app)?,
    })
}

#[tauri::command]
pub(crate) async fn plugin_native_video_status(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
) -> Result<VideoStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        status(&app, session(&app, &plugin_id, &module_id)?.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn plugin_native_video_attach(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
    options: Value,
) -> Result<VideoStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _media = MEDIA
            .try_lock()
            .map_err(|_| "native video operation is busy")?;
        let session = session(&app, &plugin_id, &module_id)?
            .ok_or("start the installed native module first")?;
        if session
            .attachment
            .lock()
            .map_err(|_| "native attachment unavailable")?
            .attached()
        {
            return Err("native video is already attached".into());
        }
        let result = (|| {
            let mut slot = session
                .attachment
                .lock()
                .map_err(|_| "native attachment unavailable")?;
            if !session.running() {
                return Err("native session has exited".into());
            }
            let (options, normalize) = frame_options(options)?;
            let normalize = filter::validate_media(&app, normalize)?;
            let cache = app
                .path()
                .app_cache_dir()
                .map_err(|e| e.to_string())?
                .join("native-video");
            let scripts = cache.join("scripts");
            std::fs::create_dir_all(&scripts).map_err(|e| e.to_string())?;
            crate::native_runtime::pin_vsscript(
                &session.launch.package_root,
                &cache.join("runtimes"),
            )?;
            let opened =
                tauri::async_runtime::block_on(session.request("frames.open", options, 5000))?;
            if opened["protocol"] != "openplayer-frame-experimental-v1" {
                return Err("unsupported native frame protocol".into());
            }
            let endpoint = endpoint(opened["endpoint"].clone())?;
            if !session.running() {
                return Err("native session stopped during attachment".into());
            }
            let mut filter = filter::OwnedFilter::prepare(app.clone(), &scripts, &endpoint)?;
            filter.normalize_to_sdr(normalize);
            let filter = Arc::new(filter);
            let cleanup = filter.clone();
            slot.mount(|| filter.install(), Box::new(move || cleanup.remove()))?;
            Ok(())
        })();
        if let Err(error) = result {
            session
                .stop_and_wait()
                .map_err(|cleanup| format!("{error}; cleanup: {cleanup}"))?;
            return Err(error);
        }
        status(&app, Some(&session))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn plugin_native_video_detach(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
) -> Result<VideoStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _media = MEDIA
            .try_lock()
            .map_err(|_| "native video operation is busy")?;
        let session = session(&app, &plugin_id, &module_id)?;
        if let Some(session) = &session {
            session
                .attachment
                .lock()
                .map_err(|_| "native attachment unavailable")?
                .detach()?;
            if session.running() {
                tauri::async_runtime::block_on(session.request("frames.close", Value::Null, 5000))?;
            }
        }
        status(&app, session.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn plugin_native_video_refresh_paused(
    app: AppHandle,
    plugin_id: String,
    module_id: String,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _media = MEDIA
            .try_lock()
            .map_err(|_| "native video operation is busy")?;
        let session = session(&app, &plugin_id, &module_id)?
            .ok_or("start the installed native module first")?;
        let slot = session
            .attachment
            .lock()
            .map_err(|_| "native attachment unavailable")?;
        if !session.running() || !slot.attached() || !filter::enabled(&app)? {
            return Err("native video is not attached".into());
        }
        filter::refresh_paused(&app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn conversion_is_host_owned_and_bounded() {
        assert_eq!(frame_options(Value::Null).unwrap(), (Value::Null, false));
        assert_eq!(
            frame_options(json!({"inputConversion":"sdr-bt709", "settings":{"intensity":1}}))
                .unwrap(),
            (json!({"settings":{"intensity":1}}), true)
        );
        for value in [
            json!(3),
            json!({"inputConversion":null}),
            json!({"inputConversion":"lavfi=arbitrary"}),
        ] {
            assert!(frame_options(value).is_err());
        }
    }
    #[test]
    fn endpoint_rejects_extra_fields_invalid_ports_and_tokens() {
        assert!(endpoint(json!({"port":1234,"token":"a".repeat(64)})).is_ok());
        for value in [
            json!({"port":0,"token":"a".repeat(64)}),
            json!({"port":65536,"token":"a".repeat(64)}),
            json!({"port":1,"token":"x".repeat(64)}),
            json!({"port":1,"token":"a".repeat(64),"host":"remote"}),
        ] {
            assert!(endpoint(value).is_err());
        }
    }
}

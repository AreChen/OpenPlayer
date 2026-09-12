//! Session-owned video filters and native presentation, separate from plan validation.
use super::{REGISTRY, Session};
use crate::{appearance_store::AppearanceStoreState, mpv_embed::native_video_filter as filter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

static MEDIA: Mutex<()> = Mutex::new(());
static MEDIA_TEARDOWN: AtomicBool = AtomicBool::new(false);

pub(crate) struct MediaChangeGuard {
    _lock: MutexGuard<'static, ()>,
}
impl Drop for MediaChangeGuard {
    fn drop(&mut self) {
        MEDIA_TEARDOWN.store(false, Ordering::Release);
    }
}
pub(crate) fn media_teardown_active() -> bool {
    MEDIA_TEARDOWN.load(Ordering::Acquire)
}

// Order: media -> registry -> attachment -> mpv. Media replacement retains this
// guard through player creation, so no attachment can bind to the outgoing player.
pub(crate) fn media_change_guard() -> Result<MediaChangeGuard, String> {
    let guard = MEDIA
        .lock()
        .map_err(|_| "native video operation unavailable")?;
    let guard = MediaChangeGuard { _lock: guard };
    MEDIA_TEARDOWN.store(true, Ordering::Release);
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

fn frame_options(mut options: Value) -> Result<(Value, bool, Option<f64>), String> {
    let rate = options
        .as_object_mut()
        .and_then(|object| object.remove("frameRateLimit"));
    let rate = match rate {
        None => None,
        Some(value) => Some(
            value
                .as_f64()
                .filter(|fps| fps.is_finite() && (1.0..=120.0).contains(fps))
                .ok_or("frameRateLimit must be a number from 1 to 120")?,
        ),
    };
    let conversion = match &mut options {
        Value::Object(object) => object.remove("inputConversion"),
        Value::Null => None,
        _ => return Err("native video options must be an object".into()),
    };
    let normalize = crate::mpv_embed::native_video_color::conversion_requested(conversion)?;
    Ok((options, normalize, rate))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VideoStatus {
    supported: bool,
    running: bool,
    attached: bool,
    filter_enabled: bool,
    presentation_active: bool,
}

fn session(app: &AppHandle, plugin: &str, module: &str) -> Result<Option<Arc<Session>>, String> {
    let modules = app.state::<AppearanceStoreState>().native_modules(plugin)?;
    if !modules.iter().any(|m| {
        m.id == module
            && matches!(
                m.video_adapter.as_deref(),
                Some("vapoursynth-rgb-v1" | "present-rgba-v1")
            )
    }) {
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
        filter_enabled: attached
            && session.is_some_and(|s| {
                s.launch.module.video_adapter.as_deref() == Some("vapoursynth-rgb-v1")
            })
            && filter::enabled(app)?,
        presentation_active: attached
            && session.is_some_and(is_presenter)
            && crate::mpv_embed::native_presentation::active(app)?,
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
            if is_presenter(&session) {
                return super::presentation::attach(&app, &session, &mut slot, options);
            }
            let (options, normalize, rate) = frame_options(options)?;
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
            filter.limit_frame_rate(rate);
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
                if is_presenter(session) {
                    super::presentation::wait_phase(session, "closed", 5)?;
                }
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
        let enabled = if is_presenter(&session) {
            crate::mpv_embed::native_presentation::active(&app)?
        } else {
            filter::enabled(&app)?
        };
        if !session.running() || !slot.attached() || !enabled {
            return Err("native video is not attached".into());
        }
        filter::refresh_paused(&app)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn is_presenter(session: &Arc<Session>) -> bool {
    session.launch.module.video_adapter.as_deref() == Some("present-rgba-v1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn conversion_is_host_owned_and_bounded() {
        assert_eq!(
            frame_options(Value::Null).unwrap(),
            (Value::Null, false, None)
        );
        assert_eq!(
            frame_options(json!({"inputConversion":"sdr-bt709", "settings":{"intensity":1}}))
                .unwrap(),
            (json!({"settings":{"intensity":1}}), true, None)
        );
        for value in [
            json!(3),
            json!({"inputConversion":null}),
            json!({"inputConversion":"lavfi=arbitrary"}),
            json!({"frameRateLimit":0}),
            json!({"frameRateLimit":121}),
            json!({"frameRateLimit":true}),
            json!({"frameRateLimit":"15"}),
            json!({"frameRateLimit":null}),
        ] {
            assert!(frame_options(value).is_err());
        }
        assert_eq!(
            frame_options(json!({"frameRateLimit":15,"settings":{}})).unwrap(),
            (json!({"settings":{}}), false, Some(15.0))
        );
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

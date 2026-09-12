//! Opt-in developer smoke fixture. Not an IPC command or a public plugin API.
#[cfg(windows)]
pub(crate) use super::native_video_filter as owned;
use super::*;

#[cfg(windows)]
pub(crate) fn video_diagnostics(app: &AppHandle) -> Result<Value, String> {
    with_player(app.state::<MpvEmbedState>().inner(), |player| {
        let mut result = serde_json::Map::new();
        for key in [
            "time-pos",
            "seeking",
            "avsync",
            "frame-drop-count",
            "video-out-params",
        ] {
            result.insert(
                key.into(),
                serde_json::to_value(player.mpv.get_property::<String>(key).ok())
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(Value::Object(result))
    })
}

pub(crate) async fn command(app: AppHandle, name: &str, args: Vec<String>) -> Result<(), String> {
    let name = name.to_owned();
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let args = args.iter().map(String::as_str).collect::<Vec<_>>();
            player
                .mpv
                .command(&name, &args)
                .map_err(|error| error.to_string())
        })
    })
    .await
}

pub(crate) async fn attach(app: AppHandle, script: String) -> Result<(), String> {
    #[cfg(windows)]
    if let Some(source) = std::env::var_os("OPENPLAYER_SMOKE_RUNTIME_BUNDLE") {
        let cache = std::env::var_os("OPENPLAYER_SMOKE_RUNTIME_CACHE")
            .ok_or("portable smoke requires an explicit test cache directory")?;
        let path = tauri::async_runtime::spawn_blocking(move || {
            crate::native_runtime::pin_vsscript(&PathBuf::from(source), &PathBuf::from(cache))
        })
        .await
        .map_err(|e| e.to_string())??;
        println!("PASS: host-owned runtime {}", path.display());
        if let Some(conflict) = std::env::var_os("OPENPLAYER_SMOKE_RUNTIME_CONFLICT") {
            let cache = std::env::var_os("OPENPLAYER_SMOKE_RUNTIME_CACHE")
                .ok_or("portable smoke cache missing")?;
            let result = tauri::async_runtime::spawn_blocking(move || {
                crate::native_runtime::pin_vsscript(&PathBuf::from(conflict), &PathBuf::from(cache))
            })
            .await
            .map_err(|e| e.to_string())?;
            if result
                .err()
                .is_none_or(|error| !error.contains("different video runtime"))
            {
                return Err("resident runtime conflict was not rejected".into());
            }
            println!("PASS: conflicting resident runtime rejected");
        }
    }
    let path = PathBuf::from(&script);
    if !path.is_absolute() || !path.is_file() || path.extension().is_none_or(|ext| ext != "vpy") {
        return Err("native filter smoke requires an absolute local .vpy fixture".into());
    }
    let filter = format!(
        "@native-smoke:vapoursynth=file=%{}%{}:concurrent-frames=1",
        script.len(),
        script
    );
    command(app, "vf", vec!["add".into(), filter]).await
}

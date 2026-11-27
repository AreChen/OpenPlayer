use std::{path::PathBuf, time::Duration};

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::{
    media_paths::StartupMediaState, mpv_embed::MpvEmbedState, platform_support::platform_support,
    updater,
};

use super::{CheckResult, app_update_check, app_update_download, app_update_install};

#[derive(PartialEq, Eq)]
enum FixtureAction {
    Check,
    Download,
    Install,
}

pub(crate) fn start_fixture_test(app: &AppHandle) {
    if !updater::test_updates_enabled() {
        return;
    }
    let action = match std::env::var("OPENPLAYER_TEST_UPDATE_ACTION").as_deref() {
        Ok("check") => FixtureAction::Check,
        Ok("download") => FixtureAction::Download,
        Ok("install") => FixtureAction::Install,
        _ => return,
    };
    let Some(result_path) = std::env::var_os("OPENPLAYER_TEST_UPDATE_RESULT")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    else {
        return;
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        #[cfg(target_os = "linux")]
        let support = app
            .get_webview_window("main")
            .and_then(|window| serde_json::to_value(platform_support(window)).ok())
            .unwrap_or_default();
        #[cfg(not(target_os = "linux"))]
        let support = serde_json::to_value(platform_support()).unwrap_or_default();
        if support["mpvEmbedVideo"].as_bool().unwrap_or(false)
            && !app.state::<StartupMediaState>().paths().is_empty()
        {
            let _ = tokio::time::timeout(Duration::from_secs(10), async {
                while !is_playing(&app) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            })
            .await;
        }
        let playing = is_playing(&app);
        let mut target_version = None;
        let result = async {
            let check = app_update_check(app.clone()).await?;
            if let CheckResult::Available { update } = check {
                target_version = Some(update.version.clone());
                if action == FixtureAction::Check {
                    return Ok(json!({ "status": "available" }));
                }
                app_update_download(app.clone(), update.version.clone()).await?;
                if action == FixtureAction::Download {
                    return Ok(json!({ "status": "ready" }));
                }
                let installed = app_update_install(app.clone(), update.version).await?;
                return serialize_install_result(&app, installed);
            }
            serde_json::to_value(check).map_err(|error| error.to_string())
        }
        .await;
        let (mut report, error) = match result {
            Ok(report) => (report, None),
            Err(error) => (json!({ "status": "failed" }), Some(error)),
        };
        report["version"] = json!(target_version);
        report["error"] = json!(error);
        report["currentVersion"] = json!(env!("CARGO_PKG_VERSION"));
        report["playing"] = json!(playing);
        report["displayServer"] = support["displayServer"].clone();
        report["mpvEmbedVideo"] = support["mpvEmbedVideo"].clone();
        let write = (|| -> anyhow::Result<()> {
            let directory = result_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid result path"))?;
            let mut file = tempfile::NamedTempFile::new_in(directory)?;
            serde_json::to_writer(&mut file, &report)?;
            file.persist_noclobber(result_path)?;
            Ok(())
        })();
        if let Err(error) = write {
            eprintln!("could not write the update fixture result: {error}");
        }
    });
}

fn serialize_install_result(
    app: &AppHandle,
    installed: super::InstallResult,
) -> Result<serde_json::Value, String> {
    let report = serde_json::to_value(installed).map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    {
        let mut report = report;
        let updater = app.state::<super::DesktopUpdater>();
        let stage = updater.0.try_lock().map_err(|error| error.to_string())?;
        if let super::UpdateStage::Ready(_, download) = &*stage {
            report["path"] = json!(download.path());
        }
        Ok(report)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(report)
    }
}

fn is_playing(app: &AppHandle) -> bool {
    let state = app.state::<MpvEmbedState>();
    state.player.lock().is_ok_and(|player| {
        player.as_ref().is_some_and(|player| {
            player.mpv.get_property::<f64>("time-pos").unwrap_or(0.0) > 0.0
                && !player.mpv.get_property::<bool>("pause").unwrap_or(true)
        })
    })
}

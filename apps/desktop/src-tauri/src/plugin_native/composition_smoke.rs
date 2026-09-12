//! Opt-in NR upstream for the presentation smoke harness; host shutdown owns cleanup.
use super::{REGISTRY, Session};
use crate::appearance_store::smoke::Package;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

const CALL_TIMEOUT_MS: u64 = 5000;

pub(crate) struct Upstream {
    app: AppHandle,
    _package: Package,
    session: Arc<Session>,
    gpu_uuid: String,
    worker_pid: u64,
}

fn source_path(name: &str, extension: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(std::env::var_os(name).ok_or_else(|| format!("provide {name}"))?);
    if !path.is_absolute()
        || !path.is_file()
        || !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        return Err(format!(
            "{name} must point to an existing absolute .{extension} file"
        ));
    }
    Ok(path)
}

impl Upstream {
    pub(crate) fn start(app: &AppHandle) -> Result<Self, String> {
        let source = source_path("OPENPLAYER_SMOKE_NR_PACKAGE", "opplugin")?;
        let library = source_path("OPENPLAYER_SMOKE_NVIDIA_RUNTIME", "dll")?;
        let gpu_uuid = std::env::var("OPENPLAYER_SMOKE_NR_GPU_UUID")
            .map_err(|_| "provide OPENPLAYER_SMOKE_NR_GPU_UUID; no default GPU is allowed")?;
        if !gpu_uuid.starts_with("GPU-") || gpu_uuid.len() != 40 || gpu_uuid.trim() != gpu_uuid {
            return Err("OPENPLAYER_SMOKE_NR_GPU_UUID must be an explicit NVIDIA GPU UUID".into());
        }
        let digest = format!(
            "{:x}",
            Sha256::digest(std::fs::read(&library).map_err(|e| e.to_string())?)
        );
        let package = Package::install(&source, app)?;
        let launch = package.launch()?;
        if launch.module.id != "enhance"
            || launch.module.video_adapter.as_deref() != Some("vapoursynth-rgb-v1")
        {
            return Err(
                "NR package must declare enhance with the vapoursynth-rgb-v1 adapter".into(),
            );
        }
        let session = tauri::async_runtime::block_on(async {
            let session = {
                let mut registry = REGISTRY.lock().map_err(|_| "NR registry unavailable")?;
                let key = (launch.plugin_id.clone(), launch.module.id.clone());
                if registry.sessions.contains_key(&key) {
                    return Err("NR upstream session is already registered".to_string());
                }
                let session = Session::spawn(launch)?;
                registry.sessions.insert(key, session.clone());
                session
            };
            session.initialize().await?;
            session
                .request(
                    "runtime.configure",
                    json!({"path": library, "sha256": digest}),
                    CALL_TIMEOUT_MS,
                )
                .await?;
            Ok::<_, String>(session)
        })?;
        let mut upstream = Self {
            app: app.clone(),
            _package: package,
            session,
            gpu_uuid,
            worker_pid: 0,
        };
        tauri::async_runtime::block_on(super::plugin_native_video_attach(
            app.clone(),
            upstream.session.launch.plugin_id.clone(),
            upstream.session.launch.module.id.clone(),
            json!({
                "inputConversion": "none",
                "frameRateLimit": 15,
                "settings": {"gpu_uuid": upstream.gpu_uuid, "intensity": 0.5}
            }),
        ))?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("NR worker did not become ready within 10 seconds".into());
            }
            let status =
                upstream.status((remaining.as_millis() as u64).clamp(1, CALL_TIMEOUT_MS))?;
            if Instant::now() >= deadline {
                return Err(format!("NR worker readiness timed out: {status}"));
            }
            if let Some(worker) = status["workerPid"].as_u64().filter(|pid| *pid > 0)
                && let Some(uuid) = status["device"]["uuid"].as_str()
            {
                if !uuid.eq_ignore_ascii_case(&upstream.gpu_uuid) {
                    return Err(format!("NR processing GPU identity mismatch: {status}"));
                }
                upstream.worker_pid = worker;
                break;
            }
            thread::sleep(
                Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        upstream.validate()?;
        println!("PASS: NR upstream attached on the explicitly selected GPU");
        Ok(upstream)
    }

    fn status(&self, timeout_ms: u64) -> Result<Value, String> {
        if !self.session_alive() {
            return Err("NR upstream session has exited".into());
        }
        let status = tauri::async_runtime::block_on(self.session.request(
            "frames.status",
            Value::Null,
            timeout_ms,
        ))?;
        if !status["error"].is_null() || status["running"] != true {
            return Err(format!("NR frame owner is not healthy: {status}"));
        }
        Ok(status)
    }

    pub fn validate(&self) -> Result<(), String> {
        let status = self.status(CALL_TIMEOUT_MS)?;
        if status["workerPid"].as_u64() != Some(self.worker_pid)
            || !status["device"]["uuid"]
                .as_str()
                .is_some_and(|uuid| uuid.eq_ignore_ascii_case(&self.gpu_uuid))
        {
            return Err(format!("NR worker or processing GPU changed: {status}"));
        }
        {
            let registry = REGISTRY.lock().map_err(|_| "NR registry unavailable")?;
            let key = (
                self.session.launch.plugin_id.clone(),
                self.session.launch.module.id.clone(),
            );
            if !registry
                .sessions
                .get(&key)
                .is_some_and(|session| Arc::ptr_eq(session, &self.session))
            {
                return Err("NR upstream session is no longer registered".into());
            }
        }
        let video = tauri::async_runtime::block_on(super::plugin_native_video_status(
            self.app.clone(),
            self.session.launch.plugin_id.clone(),
            self.session.launch.module.id.clone(),
        ))?;
        let video = serde_json::to_value(video).map_err(|e| e.to_string())?;
        if video["running"] != true || video["attached"] != true || video["filterEnabled"] != true {
            return Err(format!("NR native.video.status is not enabled: {video}"));
        }
        // Includes processedFrames, generation, revision and settings when provided by NR.
        println!(
            "NR_COMPOSITION_STATUS {}",
            json!({"frames": status, "video": video})
        );
        Ok(())
    }

    pub fn update_paused(&self) -> Result<(), String> {
        self.validate()?;
        let updated = tauri::async_runtime::block_on(self.session.request(
            "frames.update",
            json!({"settings": {"intensity": 4.0}}),
            CALL_TIMEOUT_MS,
        ))?;
        if updated["applied"] != true || !updated["error"].is_null() {
            return Err(format!("NR paused settings were not applied: {updated}"));
        }
        if !tauri::async_runtime::block_on(super::plugin_native_video_refresh_paused(
            self.app.clone(),
            self.session.launch.plugin_id.clone(),
            self.session.launch.module.id.clone(),
        ))? {
            return Err("NR native.video.refreshPaused did not refresh a paused frame".into());
        }
        println!("NR_COMPOSITION_UPDATE {updated}");
        self.validate()
    }

    pub fn session_alive(&self) -> bool {
        self.session.running()
    }

    pub fn processed_frames(&self) -> Result<u64, String> {
        self.status(CALL_TIMEOUT_MS)?["processedFrames"]
            .as_u64()
            .ok_or_else(|| "NR processed-frame counter missing".into())
    }

    pub fn verify_stopped(&self) -> Result<(), String> {
        self.session.tree.wait_stopped()?;
        println!("PASS: NR upstream process tree stopped after host cleanup");
        Ok(())
    }
}

// No Drop cleanup: REGISTRY retains the session until the host shuts it down.

//! Real windows + installed package + registry lifecycle. No IPC or consent bypass
//! is compiled into a normal player; this explicitly opted-in harness is trusted.
use super::{REGISTRY, Session};
use crate::{
    appearance_store::smoke::Package,
    mpv_embed::{
        self,
        native_filter_smoke::{self, owned},
    },
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

static EXIT_RUN: Mutex<Option<Run>> = Mutex::new(None);

pub(crate) fn verify_exit() -> Result<(), String> {
    let run = EXIT_RUN
        .lock()
        .map_err(|_| "exit fixture unavailable")?
        .take();
    if let Some(run) = run {
        if !run.session.tree.stopped() {
            return Err("application exit did not stop the native job".into());
        }
        run.session.tree.wait_stopped()?;
        if !REGISTRY
            .lock()
            .map_err(|_| "registry unavailable")?
            .sessions
            .is_empty()
        {
            return Err("application exit retained native sessions".into());
        }
        run.assert_scripts_removed()?;
        println!("PASS: active attachment app exit released filter and native job");
    }
    Ok(())
}

pub(crate) struct Run {
    app: AppHandle,
    package: Package,
    session: Arc<Session>,
    directory: PathBuf,
    installed: PathBuf,
    endpoint: Value,
}

fn path(name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(std::env::var_os(name).ok_or_else(|| format!("provide {name}"))?);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}

impl Run {
    pub(crate) fn start(app: &AppHandle) -> Result<Self, String> {
        let directory = path("OPENPLAYER_SMOKE_ATTACHMENT_WORK")?;
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let package = Package::import(
            &path("OPENPLAYER_SMOKE_RUNTIME_BUNDLE")?,
            &directory.join("store"),
            app,
        )?;
        let launch = package.launch()?;
        let installed = launch
            .executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bad package root")?
            .to_path_buf();
        if launch.module.video_adapter.is_none() {
            let cached = crate::native_runtime::pin_vsscript(&installed, &directory.join("cache"))?;
            println!("PASS: attachment runtime {}", cached.display());
        }
        let library = path("OPENPLAYER_SMOKE_NVIDIA_RUNTIME")?;
        let digest = format!(
            "{:x}",
            Sha256::digest(std::fs::read(&library).map_err(|e| e.to_string())?)
        );
        let session = tauri::async_runtime::block_on(async {
            let session = Session::spawn(launch)?;
            REGISTRY
                .lock()
                .map_err(|_| "registry unavailable")?
                .sessions
                .insert(
                    (
                        session.launch.plugin_id.clone(),
                        session.launch.module.id.clone(),
                    ),
                    session.clone(),
                );
            session.initialize().await?;
            session
                .request(
                    "runtime.configure",
                    json!({"path":library,"sha256":digest}),
                    5000,
                )
                .await?;
            Ok::<_, String>(session)
        })?;
        let frame_options = std::env::var("OPENPLAYER_SMOKE_FRAME_OPTIONS")
            .map_err(|_| "provide explicit developer frame options")?;
        let frame_options = serde_json::from_str(&frame_options)
            .map_err(|e| format!("invalid frame options: {e}"))?;
        let opened = if session.launch.module.video_adapter.is_some() {
            // The production command owns option parsing and opening the frame service.
            json!({"protocol":"openplayer-frame-experimental-v1", "endpoint":null})
        } else {
            tauri::async_runtime::block_on(session.request("frames.open", frame_options, 5000))?
        };
        if opened["protocol"] != "openplayer-frame-experimental-v1" {
            return Err("invalid frame protocol".into());
        }
        let run = Self {
            app: app.clone(),
            package,
            session,
            directory,
            installed,
            endpoint: opened["endpoint"].clone(),
        };
        run.command("vf", &["add", "@smoke-unrelated:format"])?;
        run.mount()?;
        let deadline = Instant::now() + Duration::from_secs(8);
        let status = loop {
            run.snapshot()?;
            let status = run.status()?;
            if status["workerPid"].as_u64().is_some() || Instant::now() >= deadline {
                break status;
            }
            thread::sleep(Duration::from_millis(100));
        };
        let worker = status["workerPid"].as_u64().ok_or("frame worker missing")?;
        if !status["error"].is_null() {
            return Err(format!("frame owner failed: {status}"));
        }
        run.assert_worker_in_job(worker as u32)?;
        if run.session.launch.module.video_adapter.is_some() {
            let state = tauri::async_runtime::block_on(super::plugin_native_video_status(
                app.clone(),
                run.session.launch.plugin_id.clone(),
                run.session.launch.module.id.clone(),
            ))?;
            let value = serde_json::to_value(state).map_err(|e| e.to_string())?;
            if value["filterEnabled"] != true || value["attached"] != true {
                return Err(format!(
                    "public video status disagrees with active processing: {value}"
                ));
            }
            println!("PASS: public video status reports an enabled attachment");
            let duplicate = tauri::async_runtime::block_on(super::plugin_native_video_attach(
                app.clone(),
                run.session.launch.plugin_id.clone(),
                run.session.launch.module.id.clone(),
                json!({}),
            ));
            if duplicate.is_ok() || !run.session.running() {
                return Err(
                    "duplicate public attach replaced or stopped the active session".into(),
                );
            }
            println!("PASS: duplicate public attachment rejected without stopping processing");
        }
        println!("PASS: registered native frame worker {worker}");
        run.command("set", &["pause", "yes"])?;
        thread::sleep(Duration::from_millis(200));
        let paused = run.snapshot()?;
        thread::sleep(Duration::from_millis(200));
        let still = run.snapshot()?;
        if !still.paused || (paused.position - still.position).abs() > 0.05 {
            return Err("attachment ignored pause".into());
        }
        let seek = std::env::var("OPENPLAYER_SMOKE_SEEK_SECONDS")
            .ok()
            .map(|s| s.parse::<f64>())
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or(0.2);
        if !seek.is_finite() || !(0.0..=36000.0).contains(&seek) {
            return Err("invalid smoke seek position".into());
        }
        run.command("seek", &[&seek.to_string(), "absolute+exact"])?;
        println!("TRACE: seeking to {seek}");
        thread::sleep(Duration::from_millis(1200));
        if (run.snapshot()?.position - seek).abs() > 0.15 {
            return Err("attachment ignored seek".into());
        }
        run.session
            .attachment
            .lock()
            .map_err(|_| "attachment unavailable")?
            .detach()?;
        run.assert_detached()?;
        run.mount()?;
        run.command("set", &["pause", "no"])?;
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let diagnostic = native_filter_smoke::video_diagnostics(&run.app)?;
            println!("VIDEO_DIAGNOSTICS {diagnostic}");
            let pts = diagnostic["time-pos"]
                .as_str()
                .and_then(|value| value.parse::<f64>().ok());
            let output: Value =
                serde_json::from_str(diagnostic["video-out-params"].as_str().unwrap_or("null"))
                    .unwrap_or(Value::Null);
            if diagnostic["seeking"] == "no"
                && output["pixelformat"] == "yuv420p"
                && pts.is_some_and(|pts| pts > seek + 0.04 && pts < seek + 10.0)
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err("enhanced video frame did not reach the seek target".into());
            }
            thread::sleep(Duration::from_millis(300));
        }
        if run.status()?["workerPid"].as_u64() != Some(worker) {
            return Err("reattach replaced the frame worker".into());
        }
        run.screenshot("processed.png")?;
        if std::env::var_os("OPENPLAYER_SMOKE_PAUSED_PREVIEW").is_some() {
            run.verify_paused_preview(worker)?;
        }
        println!("PASS: session attachment pause seek detach reattach reused worker");
        Ok(run)
    }

    fn mount(&self) -> Result<(), String> {
        if self.session.launch.module.video_adapter.is_some() {
            let options: Value = serde_json::from_str(
                &std::env::var("OPENPLAYER_SMOKE_FRAME_OPTIONS").map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            tauri::async_runtime::block_on(super::plugin_native_video_attach(
                self.app.clone(),
                self.session.launch.plugin_id.clone(),
                self.session.launch.module.id.clone(),
                options,
            ))?;
            println!("PASS: public native video attachment command");
            return Ok(());
        }
        let mut slot = self
            .session
            .attachment
            .lock()
            .map_err(|_| "attachment unavailable")?;
        if !self.session.running() {
            return Err("native session exited before attachment".into());
        }
        let filter = Arc::new(owned::OwnedFilter::prepare(
            self.app.clone(),
            &self.directory,
            &self.endpoint,
        )?);
        let cleanup = filter.clone();
        slot.mount(|| filter.install(), Box::new(move || cleanup.remove()))
    }

    fn verify_paused_preview(&self, worker: u64) -> Result<(), String> {
        let refresh = || {
            tauri::async_runtime::block_on(super::plugin_native_video_refresh_paused(
                self.app.clone(),
                self.session.launch.plugin_id.clone(),
                self.session.launch.module.id.clone(),
            ))
        };
        if refresh()? {
            return Err("refresh must not seek during playback".into());
        }
        self.command("set", &["pause", "yes"])?;
        thread::sleep(Duration::from_millis(300));
        let before = self.snapshot()?;
        for (index, intensity) in [0.0, 5.0].into_iter().enumerate() {
            let generation = self.status()?["generation"]
                .as_u64()
                .ok_or("missing generation")?;
            let updated = tauri::async_runtime::block_on(self.session.request(
                "frames.update",
                json!({"settings":{"intensity":intensity}}),
                5000,
            ))?;
            if updated["applied"] != true || !refresh()? {
                return Err("paused refresh not queued".into());
            }
            let deadline = Instant::now() + Duration::from_secs(8);
            loop {
                let status = self.status()?;
                let diagnostics = native_filter_smoke::video_diagnostics(&self.app)?;
                if status["generation"]
                    .as_u64()
                    .is_some_and(|g| g > generation)
                    && diagnostics["seeking"] == "no"
                {
                    break;
                }
                if Instant::now() > deadline {
                    return Err(format!("paused refresh timed out: {diagnostics}"));
                }
                thread::sleep(Duration::from_millis(100));
            }
            let after = self.snapshot()?;
            if !after.paused
                || (after.position - before.position).abs() > 0.02
                || self.status()?["workerPid"].as_u64() != Some(worker)
            {
                return Err("paused refresh changed position, pause or worker".into());
            }
            self.screenshot(&format!("paused-{index}.png"))?;
        }
        self.command("set", &["pause", "no"])?;
        println!(
            "PASS: paused settings refresh preserved position and worker; playback refresh was a no-op"
        );
        Ok(())
    }

    fn command(&self, name: &str, args: &[&str]) -> Result<(), String> {
        tauri::async_runtime::block_on(native_filter_smoke::command(
            self.app.clone(),
            name,
            args.iter().map(|s| (*s).into()).collect(),
        ))
    }

    fn snapshot(&self) -> Result<mpv_embed::MpvEmbedSnapshot, String> {
        tauri::async_runtime::block_on(mpv_embed::mpv_embed_snapshot(self.app.clone()))?
            .ok_or("player disappeared".into())
    }

    fn status(&self) -> Result<Value, String> {
        tauri::async_runtime::block_on(self.session.request("frames.status", Value::Null, 5000))
    }

    fn screenshot(&self, name: &str) -> Result<(), String> {
        self.command(
            "screenshot-to-file",
            &[
                self.directory
                    .join(name)
                    .to_str()
                    .ok_or("bad screenshot path")?,
                "video",
            ],
        )
    }

    fn assert_detached(&self) -> Result<(), String> {
        let filters = owned::filters(&self.app)?;
        if filters.contains("native-owned-")
            || filters.contains("native-input-")
            || !filters.contains("smoke-unrelated")
        {
            return Err(format!(
                "attachment cleanup damaged filter ownership: {filters}"
            ));
        }
        Ok(())
    }

    fn assert_scripts_removed(&self) -> Result<(), String> {
        let directory = if self.session.launch.module.video_adapter.is_some() {
            self.app
                .path()
                .app_cache_dir()
                .map_err(|e| e.to_string())?
                .join("native-video/scripts")
        } else {
            self.directory.clone()
        };
        for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
            if entry
                .map_err(|e| e.to_string())?
                .path()
                .extension()
                .is_some_and(|ext| ext == "vpy")
            {
                return Err("attachment cleanup retained a generated script".into());
            }
        }
        println!("PASS: actual host attachment script directory is clean");
        Ok(())
    }

    fn assert_worker_in_job(&self, pid: u32) -> Result<(), String> {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::{
                JobObjects::IsProcessInJob,
                Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
            },
        };
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return Err("frame worker unavailable".into());
        }
        let mut inside = 0;
        let result =
            unsafe { IsProcessInJob(process, self.session.tree.job_handle(), &mut inside) };
        unsafe {
            CloseHandle(process);
        }
        if result == 0 || inside == 0 {
            return Err("frame worker escaped session job".into());
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<(), String> {
        let action =
            std::env::var("OPENPLAYER_SMOKE_NATIVE_ATTACHMENT").map_err(|e| e.to_string())?;
        if action == "app-exit" {
            *EXIT_RUN.lock().map_err(|_| "exit fixture unavailable")? = Some(self);
            println!("TRACE: retaining active attachment until application exit");
            return Ok(());
        }
        println!("TRACE: beginning attachment {action}");
        match action.as_str() {
            "detach" => {
                let state = tauri::async_runtime::block_on(super::plugin_native_video_detach(
                    self.app.clone(),
                    self.session.launch.plugin_id.clone(),
                    self.session.launch.module.id.clone(),
                ))?;
                let state = serde_json::to_value(state).map_err(|e| e.to_string())?;
                if state["attached"] != false
                    || state["filterEnabled"] != false
                    || state["running"] != true
                {
                    return Err(format!("public detach state mismatch: {state}"));
                }
                println!("PASS: public detach removed processing and retained control module");
                tauri::async_runtime::block_on(super::plugin_native_stop(
                    self.session.launch.plugin_id.clone(),
                    None,
                ))?;
            }
            "media-change" => {
                let path = path("OPENPLAYER_SMOKE_REPLACEMENT_MEDIA")?
                    .to_string_lossy()
                    .into_owned();
                crate::window::smoke::on_main(&self.app, move |app| {
                    crate::window::mpv_overlay_open_path(
                        app.clone(),
                        app.state::<mpv_embed::MpvEmbedState>(),
                        path,
                        None,
                        Some(0.0),
                        None,
                    )
                    .map(|_| ())
                })?;
                println!("PASS: media replacement invalidated native video session");
            }
            "stop" => tauri::async_runtime::block_on(super::plugin_native_stop(
                self.session.launch.plugin_id.clone(),
                None,
            ))?,
            "crash" => {
                // Kill only the protocol parent. The real idle monitor must kill
                // its surviving GPU helper and remove the video attachment.
                tauri::async_runtime::block_on(self.session.crash_for_smoke())?;
                let deadline = Instant::now() + Duration::from_secs(5);
                while Instant::now() < deadline {
                    if self.assert_detached().is_ok() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                self.assert_detached()?;
                REGISTRY
                    .lock()
                    .map_err(|_| "registry unavailable")?
                    .prune_exited()?;
            }
            "disable" | "upgrade" | "uninstall" => self.package.action(&action)?,
            _ => return Err("unknown attachment lifecycle scenario".into()),
        }
        self.session.tree.wait_stopped()?;
        self.assert_scripts_removed()?;
        if action == "media-change" {
            if owned::filters(&self.app)?.contains("native-owned-") {
                return Err("old attachment survived media replacement".into());
            }
        } else {
            self.assert_detached()?;
        }
        if self.mount().is_ok() {
            return Err("stopped session reattached".into());
        }
        let key = (
            self.session.launch.plugin_id.clone(),
            self.session.launch.module.id.clone(),
        );
        if REGISTRY
            .lock()
            .map_err(|_| "registry unavailable")?
            .sessions
            .contains_key(&key)
        {
            return Err("stopped session remains registered".into());
        }
        if action == "uninstall" && self.installed.exists() {
            return Err("uninstalled runtime is still locked".into());
        }
        let before = self.snapshot()?.position;
        thread::sleep(Duration::from_millis(500));
        if self.snapshot()?.position <= before {
            return Err("original playback did not resume".into());
        }
        self.screenshot("recovered.png")?;
        println!(
            "PASS: attachment {action} removed owned filter, preserved unrelated filter, zero job processes, playback recovered"
        );
        Ok(())
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        // Also run on assertion/fixture failure, before the outer watchdog exits.
        let _ = super::invalidate_plugin(&self.session.launch.plugin_id);
    }
}

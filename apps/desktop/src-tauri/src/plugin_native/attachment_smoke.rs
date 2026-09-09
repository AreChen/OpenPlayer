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
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

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
        )?;
        let launch = package.launch()?;
        let installed = launch
            .executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bad package root")?
            .to_path_buf();
        let cached = crate::native_runtime::pin_vsscript(&installed, &directory.join("cache"))?;
        println!("PASS: attachment runtime {}", cached.display());
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
        let opened =
            tauri::async_runtime::block_on(session.request("frames.open", frame_options, 5000))?;
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
        println!("PASS: registered native frame worker {worker}");
        run.command("set", &["pause", "yes"])?;
        thread::sleep(Duration::from_millis(200));
        let paused = run.snapshot()?;
        thread::sleep(Duration::from_millis(200));
        let still = run.snapshot()?;
        if !still.paused || (paused.position - still.position).abs() > 0.05 {
            return Err("attachment ignored pause".into());
        }
        run.command("seek", &["0.2", "absolute+exact"])?;
        thread::sleep(Duration::from_millis(1200));
        if (run.snapshot()?.position - 0.2).abs() > 0.15 {
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
        thread::sleep(Duration::from_millis(1200));
        if run.status()?["workerPid"].as_u64() != Some(worker) {
            return Err("reattach replaced the frame worker".into());
        }
        run.screenshot("processed.png")?;
        println!("PASS: session attachment pause seek detach reattach reused worker");
        Ok(run)
    }

    fn mount(&self) -> Result<(), String> {
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
        if filters.contains("native-owned-") || !filters.contains("smoke-unrelated") {
            return Err(format!(
                "attachment cleanup damaged filter ownership: {filters}"
            ));
        }
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
        match action.as_str() {
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
        self.assert_detached()?;
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

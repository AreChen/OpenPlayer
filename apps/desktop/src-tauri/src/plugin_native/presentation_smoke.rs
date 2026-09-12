//! Opt-in real-package/native-window integration; never built into release IPC.
use super::{REGISTRY, Session};
use crate::{
    appearance_store::smoke::Package,
    mpv_embed::{self, native_presentation},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

static EXIT_SESSION: Mutex<Option<Arc<Session>>> = Mutex::new(None);
static EXIT_UPSTREAM: Mutex<Option<super::composition_smoke::Upstream>> = Mutex::new(None);
pub(crate) fn verify_exit() -> Result<(), String> {
    if let Some(session) = EXIT_SESSION
        .lock()
        .map_err(|_| "presentation fixture poisoned")?
        .take()
    {
        session.tree.wait_stopped()?;
        println!("PASS: native presenter job terminated on application exit");
    }
    if let Some(upstream) = EXIT_UPSTREAM
        .lock()
        .map_err(|_| "upstream fixture poisoned")?
        .take()
    {
        upstream.verify_stopped()?;
        println!("PASS: upstream NR job stopped with the native presenter on application exit");
    }
    Ok(())
}

pub(crate) struct Run {
    app: AppHandle,
    package: Package,
    session: Arc<Session>,
    before: Value,
    options: Value,
    upstream: Option<super::composition_smoke::Upstream>,
}
impl Run {
    pub(crate) fn start(app: &AppHandle) -> Result<Self, String> {
        let executable = PathBuf::from(
            std::env::var_os("OPENPLAYER_SMOKE_PRESENTATION")
                .ok_or("provide native presenter executable")?,
        );
        if !executable.is_absolute() || !executable.is_file() {
            return Err("presentation fixture executable must exist at an absolute path".into());
        }
        let luid = std::env::var("OPENPLAYER_SMOKE_PRESENTATION_LUID")
            .map_err(|_| "provide an explicit GPU LUID")?;
        let directory = std::env::temp_dir().join(format!(
            "openplayer-presentation-package-{}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let package = if executable.extension().is_some_and(|ext| ext == "opplugin") {
            Package::import(&executable, &directory.join("store"), app)?
        } else {
            let source = directory.join("package");
            std::fs::create_dir(&source).map_err(|e| e.to_string())?;
            std::fs::create_dir(source.join("bin")).map_err(|e| e.to_string())?;
            let parent = executable
                .parent()
                .ok_or("invalid native executable path")?;
            for file in [
                "libxess_fg.dll",
                "libxell.dll",
                "Intel-LICENSE.txt",
                "Intel-third-party-programs.txt",
            ] {
                std::fs::copy(parent.join(file), source.join("bin").join(file))
                    .map_err(|e| e.to_string())?;
            }
            let bytes = std::fs::read(&executable).map_err(|e| e.to_string())?;
            std::fs::write(source.join("bin/presenter.exe"), &bytes).map_err(|e| e.to_string())?;
            let manifest = json!({ "id":"dev.openplayer.fixture.presenter", "name":"Native Presentation Fixture", "version":"0.1.0", "apiVersion":"1", "entry":"manifest",
            "runtime":{"kind":"webviewJs","entry":"runtime.js","sandbox":"openplayer-worker"},
            "contributes":{"capabilities":[{"id":"presenter","name":"Presenter","kind":"nativeTool","permissions":["native.process","native.video"]}],
                "nativeModules":[{"id":"enhance","protocol":"openplayer-native-v1","videoAdapter":"present-rgba-v1",
                    "methods":["describe","gpu.list","frames.open","frames.status","frames.close"],
                    "targets":{"windows-x86_64":{"entry":"bin/presenter.exe","sha256":format!("{:x}",Sha256::digest(&bytes))}}}]}});
            std::fs::write(
                source.join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(
                source.join("runtime.js"),
                "// Native fixture driven by the opted-in host harness.\n",
            )
            .map_err(|e| e.to_string())?;
            Package::import(&source, &directory.join("store"), app)?
        };
        let upstream = if std::env::var_os("OPENPLAYER_SMOKE_NR_PACKAGE").is_some() {
            Some(super::composition_smoke::Upstream::start(app)?)
        } else {
            None
        };
        let session = spawn(&package)?;
        let before = native_presentation::diagnostics(app)?;
        let options = if let Ok(value) = std::env::var("OPENPLAYER_SMOKE_PRESENTATION_OPTIONS") {
            let mut value: Value = serde_json::from_str(&value).map_err(|e| e.to_string())?;
            let object = value
                .as_object_mut()
                .ok_or("presentation smoke options must be an object")?;
            object.insert("adapterLuid".into(), json!(luid));
            value
        } else {
            json!({"adapterLuid":luid,"width":960,"height":540,"frameRateLimit":30,"inputConversion":"sdr-bt709"})
        };
        let run = Self {
            app: app.clone(),
            package,
            session,
            before,
            options,
            upstream,
        };
        if std::env::var_os("OPENPLAYER_SMOKE_CLOSE_DURING_PRESENTATION_INIT").is_some() {
            *EXIT_SESSION
                .lock()
                .map_err(|_| "presentation fixture poisoned")? = Some(run.session.clone());
            let result = run.attach();
            println!(
                "TRACE: attachment returned during close: {}",
                if result.is_ok() {
                    "attached"
                } else {
                    "cancelled"
                }
            );
            thread::sleep(Duration::from_secs(10));
            return Err("application did not exit after close during initialization".into());
        }
        run.attach()?;
        run.wait_generated(20)?;
        run.switch_decoder("software", "no")?;
        let expected = if std::env::var_os("OPENPLAYER_SMOKE_COPY_DEVICE").is_some() {
            "nvdec-copy"
        } else {
            "no"
        };
        run.switch_decoder("hardware", expected)?;
        run.wait_generated(25)?;
        println!("PASS: presentation-owned software/hardware decoding switch");
        println!("PASS: installed native presenter receives frames from the existing mpv core");
        let paused = tauri::async_runtime::block_on(mpv_embed::mpv_embed_pause(app.clone()))?;
        run.switch_decoder("software", "no")?;
        run.switch_decoder("hardware", expected)?;
        let switched = tauri::async_runtime::block_on(mpv_embed::mpv_embed_snapshot(app.clone()))?
            .ok_or("player missing")?;
        if !switched.paused || (switched.position - paused.position).abs() > 0.05 {
            return Err("decoding switch changed the paused media clock".into());
        }
        println!("PASS: paused decoder switches retain the media position");
        thread::sleep(Duration::from_millis(250));
        let still = run.status()?;
        let before_hash = native_presentation::diagnostics(app)?["lastHash"].clone();
        tauri::async_runtime::block_on(mpv_embed::mpv_embed_plugin_command(
            app.clone(),
            "show-text".into(),
            json!(["Paused native presentation redraw", 10000]),
        ))?;
        wait("paused OSD redraw", || {
            Ok(native_presentation::diagnostics(app)?["lastHash"] != before_hash)
        })?;
        let after = tauri::async_runtime::block_on(mpv_embed::mpv_embed_snapshot(app.clone()))?
            .ok_or("player missing")?;
        if !after.paused
            || (after.position - paused.position).abs() > 0.05
            || run.status()?["generatedFrames"] != still["generatedFrames"]
        {
            return Err("native presenter advanced while paused".into());
        }
        if let Some(upstream) = &run.upstream {
            let before_hash = native_presentation::diagnostics(app)?["lastHash"].clone();
            upstream.update_paused()?;
            wait("paused NR update reaches presenter", || {
                Ok(native_presentation::diagnostics(app)?["lastHash"] != before_hash)
            })?;
            let after = tauri::async_runtime::block_on(mpv_embed::mpv_embed_snapshot(app.clone()))?
                .ok_or("player missing")?;
            if !after.paused || (after.position - paused.position).abs() > 0.08 {
                return Err("NR refresh advanced playback while paused".into());
            }
            println!("PASS: paused NR parameter update reaches downstream presentation pixels");
        }
        tauri::async_runtime::block_on(mpv_embed::mpv_embed_seek(
            app.clone(),
            paused.position + 1.0,
        ))?;
        thread::sleep(Duration::from_millis(250));
        tauri::async_runtime::block_on(mpv_embed::mpv_embed_play(app.clone()))?;
        let generated = run.status()?["generatedFrames"].as_u64().unwrap_or(0);
        run.wait_generated(generated + 15)?;
        println!("PASS: paused redraw, seek, resume and temporal-history invalidation");
        if run.upstream.is_some() {
            run.sample_composition()?;
        }
        Ok(run)
    }
    fn switch_decoder(&self, mode: &str, expected: &str) -> Result<(), String> {
        let before = native_presentation::diagnostics(&self.app)?;
        let snapshot = tauri::async_runtime::block_on(mpv_embed::mpv_embed_set_hwdec(
            self.app.clone(),
            mode.into(),
        ))?;
        let state = native_presentation::diagnostics(&self.app)?;
        if state["core"] != before["core"]
            || state["currentVo"] != "libmpv"
            || state["presentationActive"] != true
            || snapshot.hwdec != expected
            || state["hwdecCurrent"] != expected
        {
            return Err(format!(
                "unexpected {mode} decode state: {state}; snapshot={}",
                snapshot.hwdec
            ));
        }
        if expected == "nvdec-copy" {
            let device =
                std::env::var("OPENPLAYER_SMOKE_COPY_DEVICE").map_err(|e| e.to_string())?;
            if state["cudaDevice"] != device {
                return Err(format!("wrong hardware-copy device: {state}"));
            }
        }
        println!("TRACE: {mode} decoder {state}");
        Ok(())
    }
    fn sample_composition(&self) -> Result<(), String> {
        use crate::mpv_embed::native_filter_smoke::video_diagnostics;
        let start = Instant::now();
        let before = video_diagnostics(&self.app)?;
        let before_frames = self.status()?;
        let upstream = self.upstream.as_ref().unwrap();
        let before_processed = upstream.processed_frames()?;
        let mut max_av = 0.0f64;
        while start.elapsed() < Duration::from_secs(12) {
            self.upstream.as_ref().unwrap().validate()?;
            let value = video_diagnostics(&self.app)?;
            let av = value["avsync"]
                .as_str()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or("mpv A/V clock diagnostic missing")?;
            max_av = max_av.max(av.abs());
            thread::sleep(Duration::from_millis(250));
        }
        let elapsed = start.elapsed().as_secs_f64();
        let after = video_diagnostics(&self.app)?;
        let position = |v: &Value| {
            v["time-pos"]
                .as_str()
                .and_then(|v| v.parse::<f64>().ok())
                .ok_or("mpv time diagnostic missing")
        };
        let progression = position(&after)? - position(&before)?;
        let frames = self.status()?;
        let source = frames["sourceFrames"]
            .as_u64()
            .unwrap_or(0)
            .saturating_sub(before_frames["sourceFrames"].as_u64().unwrap_or(0));
        let processed = upstream
            .processed_frames()?
            .saturating_sub(before_processed);
        let generated = frames["generatedFrames"]
            .as_u64()
            .unwrap_or(0)
            .saturating_sub(before_frames["generatedFrames"].as_u64().unwrap_or(0));
        println!(
            "TRACE: NR plus XeFG short A/V sample {}",
            json!({"elapsedSeconds":elapsed,"mediaSeconds":progression,"maxMpvAvSyncSeconds":max_av,"nrProcessedFrames":processed,"sourceFrames":source,"generatedFrames":generated,"presenter":frames})
        );
        if std::env::var_os("OPENPLAYER_SMOKE_OVERLOAD").is_some() {
            if source < 30
                || frames["latePresentedFrames"].as_u64().unwrap_or(0) < 1
                || frames["lastSequence"] == before_frames["lastSequence"]
            {
                return Err("overloaded upstream starved presentation".into());
            }
            println!(
                "PASS: overloaded upstream keeps displaying source frames without claiming generated frames or real-time A/V"
            );
            return Ok(());
        }
        if (progression - elapsed).abs() > 1.0
            || max_av > 0.15
            || generated < 30
            || source > processed + 5
            || source as f64 / elapsed > 16.0
            || generated > source + 1
            || generated < source.saturating_mul(9) / 10
        {
            return Err("NR plus XeFG short A/V/cadence sample failed".into());
        }
        Ok(())
    }
    fn attach(&self) -> Result<(), String> {
        let state = tauri::async_runtime::block_on(super::plugin_native_video_attach(
            self.app.clone(),
            self.session.launch.plugin_id.clone(),
            self.session.launch.module.id.clone(),
            self.options.clone(),
        ))?;
        let state = serde_json::to_value(state).map_err(|e| e.to_string())?;
        if state["presentationActive"] != true || state["filterEnabled"] != false {
            return Err(format!("invalid presenter attachment status: {state}"));
        }
        let output = native_presentation::diagnostics(&self.app)?;
        if output["matrix"] != "bt.709"
            || !output["gamma"]
                .as_str()
                .is_some_and(|gamma| matches!(gamma, "bt.1886" | "bt.709" | "srgb"))
        {
            return Err(format!("presenter received non-SDR output: {output}"));
        }
        println!("TRACE: validated presentation color output {output}");
        Ok(())
    }
    fn status(&self) -> Result<Value, String> {
        tauri::async_runtime::block_on(self.session.request("frames.status", Value::Null, 5000))
    }
    fn wait_generated(&self, count: u64) -> Result<(), String> {
        let mut next_trace = Instant::now();
        wait("generated native frames", || {
            let status = self.status()?;
            if Instant::now() >= next_trace {
                println!(
                    "TRACE: generated progress {status}; host={}",
                    native_presentation::diagnostics(&self.app)?
                );
                next_trace = Instant::now() + Duration::from_secs(1);
            }
            if status["phase"] == "failed" || !status["error"].is_null() {
                return Err(format!("native renderer: {status}"));
            }
            let counter = if std::env::var_os("OPENPLAYER_SMOKE_OVERLOAD").is_some() {
                "sourceFrames"
            } else {
                "generatedFrames"
            };
            Ok(status[counter].as_u64().unwrap_or(0) >= count)
        })
    }
    fn assert_restored(&self) -> Result<(), String> {
        wait("original mpv output restored", || {
            let state = native_presentation::diagnostics(&self.app)?;
            Ok(state["core"] == self.before["core"]
                && state["vo"] == self.before["vo"]
                && state["currentVo"] == self.before["currentVo"]
                && state["hwdec"] == self.before["hwdec"]
                && state["cudaDevice"] == self.before["cudaDevice"]
                && state["matrix"] == self.before["matrix"]
                && state["gamma"] == self.before["gamma"]
                && state["filters"] == self.before["filters"]
                && state["presentationActive"] == false)
        })
    }
    pub(crate) fn finish(mut self) -> Result<(), String> {
        let count = self.status()?["generatedFrames"].as_u64().unwrap_or(0);
        self.wait_generated(count + 5)?;
        let running = native_presentation::diagnostics(&self.app)?;
        println!("TRACE: presentation after window transitions {running}");
        // Also verify that detach retains the latest user choice, not the initial
        // preference. Do not let auto-safe select a different GPU during tests.
        self.switch_decoder("software", "no")?;
        self.before["hwdec"] = json!("no");
        tauri::async_runtime::block_on(super::plugin_native_video_detach(
            self.app.clone(),
            self.session.launch.plugin_id.clone(),
            self.session.launch.module.id.clone(),
        ))?;
        self.assert_restored()?;
        if let Some(upstream) = &self.upstream {
            upstream.validate()?;
        }
        self.attach()?;
        self.wait_generated(10)?;
        tauri::async_runtime::block_on(self.session.crash_for_smoke())?;
        self.assert_restored()?;
        if let Some(upstream) = &self.upstream {
            upstream.validate()?;
        }
        self.session.stop_and_wait()?;
        println!(
            "PASS: detach and native-process crash restore original output on the same mpv core"
        );
        self.session = spawn(&self.package)?;
        self.attach()?;
        self.wait_generated(10)?;
        *EXIT_SESSION
            .lock()
            .map_err(|_| "presentation fixture poisoned")? = Some(self.session.clone());
        *EXIT_UPSTREAM
            .lock()
            .map_err(|_| "upstream fixture poisoned")? = self.upstream.take();
        println!("PASS: presenter reattached; leaving it active for real application close");
        Ok(())
    }
}
fn spawn(package: &Package) -> Result<Arc<Session>, String> {
    let launch = package.launch()?;
    tauri::async_runtime::block_on(async {
        let session = Session::spawn(launch)?;
        session.initialize().await?;
        REGISTRY
            .lock()
            .map_err(|_| "native registry unavailable")?
            .sessions
            .insert(
                (
                    session.launch.plugin_id.clone(),
                    session.launch.module.id.clone(),
                ),
                session.clone(),
            );
        Ok(session)
    })
}
fn wait(label: &str, mut check: impl FnMut() -> Result<bool, String>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if check()? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("presentation fixture timed out: {label}"));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

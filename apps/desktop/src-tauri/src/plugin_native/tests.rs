use super::*;
use serde_json::{Value, json};

#[cfg(windows)]
mod frame_owner;
#[cfg(windows)]
pub(crate) use frame_owner::exercise_frame_owner_lifecycle;

fn module() -> NativeModule {
    serde_json::from_value(json!({ "id": "enhance", "protocol": "openplayer-native-v1",
        "methods": ["echo", "delay", "fail", "crash", "oversize", "child", "exitSoon", "earlyChild"],
        "targets": { "windows-x86_64": { "entry": "bin/worker.exe", "sha256": "0".repeat(64) } }
    })).unwrap()
}

#[test]
fn requires_native_permission_and_unique_declarations() {
    let m = module();
    assert!(validate_modules(&[], &[]).is_ok());
    assert!(validate_modules(std::slice::from_ref(&m), &[]).is_err());
    assert!(validate_modules(std::slice::from_ref(&m), &["native.process".into()]).is_ok());
    assert!(validate_modules(&[m.clone(), m], &["native.process".into()]).is_err());
}

#[test]
fn rejects_unsafe_native_targets_and_methods() {
    for entry in [
        "../evil.exe",
        "C:/evil.exe",
        "/evil.exe",
        "bin\\evil.exe",
        "bin/tool.cmd",
        "a//b.exe",
        "a/./b.exe",
    ] {
        let mut m = module();
        m.targets.get_mut("windows-x86_64").unwrap().entry = entry.into();
        assert!(
            validate_modules(&[m], &["native.process".into()]).is_err(),
            "{entry}"
        );
    }
    for methods in [
        vec!["host.initialize".into()],
        vec!["a".into(), "a".into()],
        vec![],
    ] {
        let mut m = module();
        m.methods = methods;
        assert!(validate_modules(&[m], &["native.process".into()]).is_err());
    }
    let mut m = module();
    m.targets.get_mut("windows-x86_64").unwrap().sha256 = "bad".into();
    assert!(validate_modules(&[m], &["native.process".into()]).is_err());
    let mut v = serde_json::to_value(module()).unwrap();
    v["extra"] = json!(true);
    assert!(serde_json::from_value::<NativeModule>(v).is_err());
}

#[test]
fn validates_executable_integrity() {
    use sha2::{Digest, Sha256};
    let path =
        std::env::temp_dir().join(format!("openplayer-native-hash-{}.tmp", std::process::id()));
    std::fs::write(&path, b"native fixture").unwrap();
    let digest = format!("{:x}", Sha256::digest(b"native fixture"));
    assert!(verify_executable(&path, &digest).is_ok());
    assert!(verify_executable(&path, &digest.to_uppercase()).is_ok());
    assert!(verify_executable(&path, &"0".repeat(64)).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn enforces_response_framing_ids_and_exact_result_or_error() {
    use session::decode_response;
    assert_eq!(
        decode_response(b"{\"id\":1,\"result\":null}\n", 1).unwrap(),
        Value::Null
    );
    for line in [
        "",
        "{}\n",
        "{\"id\":2,\"result\":true}\n",
        "{\"id\":1,\"result\":true}",
        "{\"id\":1,\"error\":null}\n",
        "{\"id\":1,\"error\":\"bad\"}\n",
        "{\"id\":1,\"result\":null,\"error\":\"bad\"}\n",
        "{\"id\":1,\"result\":1,\"extra\":2}\n",
    ] {
        assert!(decode_response(line.as_bytes(), 1).is_err(), "{line}");
    }
    assert!(decode_response(&vec![b'\n'; session::MAX_MESSAGE_BYTES + 1], 1).is_err());
}

fn plan() -> video_plan::VideoPlan {
    let a = json!({"width":1280,"height":720,"frameRate":{"numerator":24,"denominator":1},"pixelFormat":"rgba8","colorSpace":"srgb"});
    let mut b = a.clone();
    b["width"] = json!(1920);
    b["height"] = json!(1080);
    let mut c = b.clone();
    c["frameRate"]["numerator"] = json!(48);
    serde_json::from_value(json!({"input":a,"stages":[
        {"moduleId":"enhance","input":a,"output":b,"lookaheadFrames":0,"maxOutputFrames":1},
        {"moduleId":"enhance","input":b,"output":c,"lookaheadFrames":1,"maxOutputFrames":2}
    ]}))
    .unwrap()
}

#[test]
fn composes_resolution_and_rate_changes_but_never_claims_executable() {
    let result = video_plan::validate_plan(plan(), &[module()]).unwrap();
    assert_eq!(result.output.width, 1920);
    assert_eq!(result.output.frame_rate.numerator, 48);
    assert!((result.lookahead_ms - 1000.0 / 24.0).abs() < 0.001);
    assert!(!result.executable);
    let mut p = plan();
    p.stages[1].input.frame_rate.numerator = 48;
    p.stages[1].input.frame_rate.denominator = 2;
    assert!(video_plan::validate_plan(p, &[module()]).is_ok());
}

#[test]
fn rejects_incompatible_or_unbounded_video_plans() {
    let mut p = plan();
    p.stages[1].input.width = 1280;
    assert!(video_plan::validate_plan(p, &[module()]).is_err());
    let mut p = plan();
    p.stages[1].max_output_frames = 1;
    assert!(video_plan::validate_plan(p, &[module()]).is_err());
    let mut p = plan();
    p.stages[0].lookahead_frames = 9;
    assert!(video_plan::validate_plan(p, &[module()]).is_err());
    let mut p = plan();
    p.input.frame_rate.denominator = 0;
    assert!(video_plan::validate_plan(p, &[module()]).is_err());
    let mut p = plan();
    p.stages[0].output.color_space = "bt2020-pq".into();
    assert!(video_plan::validate_plan(p, &[module()]).is_err());
    assert!(video_plan::validate_plan(plan(), &[]).is_err());
}

#[test]
#[ignore = "requires explicitly built SDK protocol-fixture via OPENPLAYER_NATIVE_TEST_EXECUTABLE"]
fn native_process_roundtrip_faults_and_lifecycle() {
    let executable = std::env::var_os("OPENPLAYER_NATIVE_TEST_EXECUTABLE")
        .expect("build SDK fixture first")
        .into();
    let launch = ModuleLaunch {
        plugin_id: "test.native.protocol".into(),
        plugin_name: "Test".into(),
        plugin_version: "1.0.0".into(),
        language_mode: "en-US".into(),
        executable,
        args: vec!["--spawn-child".into()],
        module: module(),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let session = Session::spawn(launch.clone()).unwrap();
        session.initialize().await.unwrap();
        assert!(session.running());
        let early = session
            .request("earlyChild", Value::Null, 5000)
            .await
            .unwrap();
        #[cfg(windows)]
        assert_in_job(&session, early["pid"].as_u64().unwrap() as u32);
        let value = json!({"number":42,"text":"native RPC","nested":[1,true,null]});
        assert_eq!(
            session.request("echo", value.clone(), 5000).await.unwrap(),
            value
        );
        assert!(
            session
                .request("undeclared", Value::Null, 100)
                .await
                .is_err()
        );
        assert!(session.running());
        let child = session.request("child", Value::Null, 5000).await.unwrap();
        assert!(child["pid"].as_u64().unwrap() > 0);
        REGISTRY.lock().unwrap().sessions.insert(
            (launch.plugin_id.clone(), launch.module.id.clone()),
            session.clone(),
        );
        invalidate_plugin(&launch.plugin_id).unwrap();
        assert!(!session.running());
        assert!(session.request("echo", Value::Null, 100).await.is_err());
        for method in ["delay", "fail", "crash", "oversize"] {
            let session = Session::spawn(launch.clone()).unwrap();
            session.initialize().await.unwrap();
            assert!(
                session.request(method, Value::Null, 100).await.is_err(),
                "{method}"
            );
            session.tree.wait_stopped().unwrap();
            assert!(!session.running(), "{method}");
        }
        let session = Session::spawn(launch.clone()).unwrap();
        session.initialize().await.unwrap();
        session
            .request("exitSoon", Value::Null, 5000)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
        assert!(
            session.tree.stopped(),
            "idle exit monitor must kill helpers without list/call"
        );
        session.tree.wait_stopped().unwrap();

        let session = Session::spawn(launch).unwrap();
        session.initialize().await.unwrap();
        let busy = session.clone();
        let request = tokio::spawn(async move { busy.request("delay", Value::Null, 5000).await });
        tokio::task::yield_now().await;
        assert!(
            session
                .request("echo", Value::Null, 100)
                .await
                .unwrap_err()
                .contains("busy")
        );
        session.tree.stop();
        assert!(request.await.unwrap().is_err());
        session.tree.wait_stopped().unwrap();
    });
}

#[cfg(windows)]
fn assert_in_job(session: &Session, pid: u32) {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            JobObjects::IsProcessInJob,
            Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        },
    };
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    assert!(!process.is_null());
    let mut inside = 0;
    assert_ne!(
        unsafe { IsProcessInJob(process, session.tree.job_handle(), &mut inside) },
        0
    );
    unsafe {
        CloseHandle(process);
    }
    assert_ne!(
        inside, 0,
        "startup helper must inherit the native module job"
    );
}

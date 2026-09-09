//! Explicit developer integration fixture, never compiled into the player.
use super::*;
use std::{path::PathBuf, process::Stdio};
use tokio::io::AsyncWriteExt;

pub(crate) fn exercise_frame_owner_lifecycle(lifecycle: impl FnMut(&str)) {
    exercise_frame_owner(lifecycle, None);
}

pub(crate) fn exercise_portable_frame_owner(launch: ModuleLaunch, lifecycle: impl FnMut(&str)) {
    exercise_frame_owner(lifecycle, Some(launch));
}

fn exercise_frame_owner(mut lifecycle: impl FnMut(&str), packaged: Option<ModuleLaunch>) {
    let python = PathBuf::from(
        std::env::var_os("OPENPLAYER_NATIVE_FRAME_PYTHON").expect("provide Python runtime"),
    );
    let root = PathBuf::from(
        std::env::var_os("OPENPLAYER_NATIVE_FRAME_ROOT").expect("provide frame module root"),
    );
    assert!(python.is_absolute() && python.is_file());
    assert!(root.is_absolute() && root.join("frame_module.py").is_file());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for action in ["disable", "upgrade", "uninstall"] {
            let mut definition = module();
            definition.methods = vec![
                "frames.open".into(),
                "frames.status".into(),
                "frames.close".into(),
            ];
            let launch = ModuleLaunch {
                plugin_id: "test.native.frame.owner".into(),
                plugin_name: "Frame lifecycle fixture".into(),
                plugin_version: "1.0.0".into(),
                language_mode: "en-US".into(),
                executable: python.clone(),
                args: vec![
                    "-u".into(),
                    root.join("frame_module.py").to_string_lossy().into_owned(),
                    "--host".into(),
                    root.join(".local/build/dlssnr_host_v2.dll")
                        .to_string_lossy()
                        .into_owned(),
                    "--runtime".into(),
                    root.join(".local/runtime/nvngx_dlssnr.dll")
                        .to_string_lossy()
                        .into_owned(),
                    "--log".into(),
                    root.join(format!(
                        ".local/sdk-owner-{}-{action}.log",
                        std::process::id()
                    ))
                    .to_string_lossy()
                    .into_owned(),
                ],
                module: definition,
            };
            let launch = packaged.as_ref().unwrap_or(&launch).clone();
            let session = Session::spawn(launch.clone()).unwrap();
            session.initialize().await.unwrap();
            if packaged.is_some() {
                use sha2::{Digest, Sha256};
                let library = root.join(".local/runtime/nvngx_dlssnr.dll");
                let digest = format!("{:x}", Sha256::digest(std::fs::read(&library).unwrap()));
                let configured = session
                    .request(
                        "runtime.configure",
                        json!({
                            "path": library, "sha256": digest,
                        }),
                        5000,
                    )
                    .await
                    .unwrap();
                assert_eq!(configured["configured"], true);
            }
            let frame_options = std::env::var("OPENPLAYER_NATIVE_FRAME_OPTIONS")
                .ok()
                .map(|text| serde_json::from_str(&text).expect("valid frame test options"))
                .unwrap_or(Value::Null);
            let opened = session
                .request("frames.open", frame_options, 5000)
                .await
                .unwrap();
            assert_eq!(opened["protocol"], "openplayer-frame-experimental-v1");
            let client_python = if packaged.is_some() {
                &launch.executable
            } else {
                &python
            };
            let mut client = tokio::process::Command::new(client_python)
                .arg(root.join("scripts/sdk_frame_client_probe.py"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let mut input = client.stdin.take().unwrap();
            input
                .write_all(format!("{}\n", opened["endpoint"]).as_bytes())
                .await
                .unwrap();
            drop(input);
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                client.wait_with_output(),
            )
            .await
            .unwrap()
            .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let frame: Value = serde_json::from_slice(&output.stdout).unwrap();
            let worker_pid = frame["workerPid"].as_u64().unwrap() as u32;
            assert!(frame["mean"].as_f64().unwrap() > 1.0);
            assert_in_job(&session, worker_pid);
            let status = session
                .request("frames.status", Value::Null, 5000)
                .await
                .unwrap();
            assert_eq!(status["workerPid"], frame["workerPid"]);
            REGISTRY.lock().unwrap().sessions.insert(
                (launch.plugin_id.clone(), launch.module.id.clone()),
                session.clone(),
            );
            lifecycle(action);
            assert!(!session.running(), "{action} left the frame module running");
            session.tree.wait_stopped().unwrap();
            assert!(
                session
                    .request("frames.status", Value::Null, 100)
                    .await
                    .is_err()
            );
            assert!(
                !REGISTRY
                    .lock()
                    .unwrap()
                    .sessions
                    .contains_key(&(launch.plugin_id, launch.module.id))
            );
            println!("PASS: frame owner + NGX worker stopped on {action} (worker {worker_pid})");
        }
    });
}

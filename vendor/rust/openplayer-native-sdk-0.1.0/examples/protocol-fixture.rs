// Test-only fault injection worker. Never include this binary in a plugin package.
use openplayer_native_sdk::{json, serve};
use std::{
    process::{Command, Stdio},
    time::Duration,
};

fn main() -> std::io::Result<()> {
    if std::env::args().any(|arg| arg == "--child") {
        std::thread::sleep(Duration::from_secs(60));
        return Ok(());
    }
    let early_child = if std::env::args().any(|arg| arg == "--spawn-child") {
        Some(spawn_child()?)
    } else {
        None
    };
    serve(|method, params| match method {
        "echo" => Ok(params),
        "delay" => {
            std::thread::sleep(Duration::from_secs(10));
            Ok(json!(true))
        }
        "fail" => Err("fixture failure".into()),
        "crash" => std::process::exit(7),
        "exitSoon" => {
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                std::process::exit(8);
            });
            Ok(json!(true))
        }
        "earlyChild" => Ok(json!({ "pid": early_child })),
        "oversize" => {
            println!("{}", "x".repeat(65537));
            Ok(json!(true))
        }
        "child" => Ok(json!({ "pid": spawn_child().map_err(|e| e.to_string())? })),
        _ => Err("unsupported method".into()),
    })
}

fn spawn_child() -> std::io::Result<u32> {
    let child = Command::new(std::env::current_exe()?)
        .arg("--child")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let pid = child.id();
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
    Ok(pid)
}

use super::{
    ModuleLaunch,
    process_tree::{self, ProcessTree},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};

pub(super) const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

pub(super) struct Session {
    pub launch: ModuleLaunch,
    pub tree: ProcessTree,
    io: Mutex<SessionIo>,
}

struct SessionIo {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    sequence: u64,
}

impl Session {
    pub(super) fn running(&self) -> bool {
        if self.tree.stopped() {
            return false;
        }
        if let Ok(mut io) = self.io.try_lock()
            && !matches!(io.child.try_wait(), Ok(None))
        {
            self.tree.stop();
            return false;
        }
        true
    }
    pub(super) fn spawn(launch: ModuleLaunch) -> Result<Arc<Self>, String> {
        let mut command = Command::new(&launch.executable);
        command
            .args(&launch.args)
            .current_dir(launch.executable.parent().ok_or("invalid native path")?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        // The protocol never forwards arbitrary caller command-line arguments or env.
        command.env_remove("OPENPLAYER_NATIVE_TEST_EXECUTABLE");
        process_tree::configure(&mut command);
        let mut child = command
            .spawn()
            .map_err(|e| format!("native launch failed: {e}"))?;
        let tree = process_tree::ProcessTree::attach(&child)?;
        let input = child.stdin.take().ok_or("native stdin unavailable")?;
        let output = BufReader::new(child.stdout.take().ok_or("native stdout unavailable")?);
        let session = Arc::new(Self {
            launch,
            tree,
            io: Mutex::new(SessionIo {
                child,
                input,
                output,
                sequence: 0,
            }),
        });
        let weak = Arc::downgrade(&session);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let Some(session) = weak.upgrade() else {
                    break;
                };
                if !session.running() {
                    break;
                }
            }
        });
        Ok(session)
    }

    pub(super) async fn request(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        if self.tree.stopped() {
            return Err("native module is stopped".into());
        }
        if method != "host.initialize" && !self.launch.module.methods.iter().any(|m| m == method) {
            return Err("native method is not declared by this module".into());
        }
        if serde_json::to_vec(&params)
            .map_err(|e| e.to_string())?
            .len()
            > MAX_MESSAGE_BYTES - 1024
        {
            return Err("native request exceeds the control-message limit".into());
        }
        // Reject concurrent requests instead of building an unbounded command queue.
        let mut io = self.io.try_lock().map_err(|_| "native module is busy")?;
        if self.tree.stopped() || !matches!(io.child.try_wait(), Ok(None)) {
            self.tree.stop();
            return Err("native module has exited".into());
        }
        io.sequence += 1;
        let id = io.sequence;
        let result = tokio::time::timeout(Duration::from_millis(timeout_ms), async {
            let mut bytes =
                serde_json::to_vec(&json!({ "id": id, "method": method, "params": params }))
                    .map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            io.input
                .write_all(&bytes)
                .await
                .map_err(|e| format!("native write: {e}"))?;
            io.input.flush().await.map_err(|e| e.to_string())?;
            let mut line = Vec::new();
            (&mut io.output)
                .take((MAX_MESSAGE_BYTES + 1) as u64)
                .read_until(b'\n', &mut line)
                .await
                .map_err(|e| format!("native read: {e}"))?;
            decode_response(&line, id)
        })
        .await
        .map_err(|_| "native request timed out".to_string())
        .and_then(|r| r);
        if result.is_err() {
            self.tree.stop();
            let _ = io.child.kill().await;
        }
        result
    }

    pub(super) async fn initialize(&self) -> Result<(), String> {
        let result = self
            .request(
                "host.initialize",
                json!({
                    "protocol": "openplayer-native-v1", "pluginId": self.launch.plugin_id,
                    "moduleId": self.launch.module.id, "maxMessageBytes": MAX_MESSAGE_BYTES,
                }),
                5000,
            )
            .await?;
        if result.get("protocol").and_then(Value::as_str) != Some("openplayer-native-v1") {
            self.tree.stop();
            return Err("native protocol negotiation failed".into());
        }
        Ok(())
    }
}

pub(super) fn decode_response(line: &[u8], expected_id: u64) -> Result<Value, String> {
    if line.len() > MAX_MESSAGE_BYTES || line.last() != Some(&b'\n') {
        return Err("native response is missing, oversized or incomplete".into());
    }
    let response: Response =
        serde_json::from_slice(line).map_err(|e| format!("invalid native response: {e}"))?;
    let fields: Value = serde_json::from_slice(line).map_err(|e| e.to_string())?;
    if fields.get("result").is_some() == fields.get("error").is_some() {
        return Err("native response must contain exactly one result or error".into());
    }
    if fields.get("error").is_some_and(|error| !error.is_string()) {
        return Err("native response error must be a string".into());
    }
    if response.id != expected_id {
        return Err("native response id mismatch".into());
    }
    if let Some(error) = response.error {
        return Err(format!("native module error: {error}"));
    }
    Ok(response.result.unwrap_or(Value::Null))
}

//! Bounded, sequential control RPC for trusted out-of-process OpenPlayer modules.
use serde::Deserialize;
pub use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};

#[cfg(feature = "presentation")]
pub mod presentation;

pub const PROTOCOL: &str = "openplayer-native-v1";
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: u64,
    method: String,
    params: Value,
}

/// Run on stdin/stdout. Reserve stdout for protocol messages; use stderr for logs.
/// Long work should return a job id, then expose short status/cancel methods.
pub fn serve(handler: impl FnMut(&str, Value) -> Result<Value, String>) -> io::Result<()> {
    serve_io(io::stdin().lock(), io::stdout().lock(), handler)
}

pub fn serve_io(
    mut input: impl BufRead,
    mut output: impl Write,
    mut handler: impl FnMut(&str, Value) -> Result<Value, String>,
) -> io::Result<()> {
    let mut initialized = false;
    loop {
        let mut bytes = Vec::new();
        (&mut input)
            .take((MAX_MESSAGE_BYTES + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if bytes.is_empty() {
            return Ok(());
        }
        if bytes.len() > MAX_MESSAGE_BYTES || bytes.last() != Some(&b'\n') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid control message size",
            ));
        }
        let request: Request = serde_json::from_slice(&bytes)?;
        let result = if !initialized {
            if request.method != "host.initialize" || request.params["protocol"] != PROTOCOL {
                Err("protocol negotiation required".into())
            } else {
                initialized = true;
                Ok(json!({ "protocol": PROTOCOL }))
            }
        } else if request.method.starts_with("host.") {
            Err("reserved host method".into())
        } else {
            handler(&request.method, request.params)
        };
        let failed = result.is_err();
        let response = match result {
            Ok(value) => json!({ "id": request.id, "result": value }),
            Err(error) => json!({ "id": request.id, "error": error }),
        };
        let mut bytes = serde_json::to_vec(&response)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_MESSAGE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "response exceeds control limit",
            ));
        }
        output.write_all(&bytes)?;
        output.flush()?;
        // v1 uses fail-closed sessions. Expected domain failures belong in result.
        if failed {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negotiates_then_dispatches() {
        let input = concat!(
            "{\"id\":1,\"method\":\"host.initialize\",\"params\":{\"protocol\":\"openplayer-native-v1\"}}\n",
            "{\"id\":2,\"method\":\"echo\",\"params\":null}\n"
        );
        let mut output = Vec::new();
        serve_io(input.as_bytes(), &mut output, |method, params| {
            assert_eq!(method, "echo");
            Ok(params)
        })
        .unwrap();
        let replies: Vec<Value> = output
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|b| serde_json::from_slice(b).unwrap())
            .collect();
        assert_eq!(replies[0]["result"]["protocol"], PROTOCOL);
        assert_eq!(replies[1], json!({ "id": 2, "result": null }));
    }
    #[test]
    fn rejects_oversize_and_missing_handshake() {
        assert!(
            serve_io(
                &vec![b'x'; MAX_MESSAGE_BYTES + 1][..],
                Vec::new(),
                |_, _| unreachable!()
            )
            .is_err()
        );
        let mut output = Vec::new();
        serve_io(
            &b"{\"id\":1,\"method\":\"echo\",\"params\":null}\n"[..],
            &mut output,
            |_, _| unreachable!(),
        )
        .unwrap();
        assert!(serde_json::from_slice::<Value>(&output).unwrap()["error"].is_string());
    }
}

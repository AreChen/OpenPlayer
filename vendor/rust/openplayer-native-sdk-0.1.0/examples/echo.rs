use openplayer_native_sdk::{json, serve};

fn main() -> std::io::Result<()> {
    serve(|method, params| match method {
        "echo" => Ok(params),
        "describe" => Ok(json!({ "name": "OpenPlayer native SDK example", "protocolVersion": 1 })),
        _ => Err("unsupported method".into()),
    })
}

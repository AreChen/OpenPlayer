use super::{execution::execute_plugin_network_request, types::PluginNetworkRequestArgs};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::Duration,
};

fn request_with_response(
    response: Vec<u8>,
    response_type: &str,
) -> Result<super::types::PluginNetworkResponse, String> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/test", listener.local_addr().unwrap());
    let (finished, completion) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
            assert!(request.len() <= 8192);
        }
        let _ = stream.write_all(&response);
        // Keep the response open: the client must reject oversized chunks before EOF.
        let _ = completion.recv_timeout(Duration::from_secs(5));
    });
    let args: PluginNetworkRequestArgs = serde_json::from_value(serde_json::json!({
        "url": url, "timeoutMs": 1000, "responseType": response_type,
    }))
    .unwrap();
    let result = tauri::async_runtime::block_on(execute_plugin_network_request(
        std::env::temp_dir(),
        "test.plugin".into(),
        args,
    ));
    let _ = finished.send(());
    server.join().unwrap();
    result
}

#[test]
fn rejects_oversized_chunked_response_without_waiting_for_eof() {
    let mut response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    for _ in 0..17 {
        response.extend_from_slice(b"10000\r\n");
        response.extend_from_slice(&vec![b'x'; 65536]);
        response.extend_from_slice(b"\r\n");
    }
    let error = request_with_response(response, "text").err().unwrap();
    assert!(error.contains("response is too large"), "{error}");
}

#[test]
fn rejects_oversized_content_length_before_reading_the_body() {
    let error = request_with_response(
        b"HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\n\r\n".to_vec(),
        "text",
    )
    .err()
    .unwrap();
    assert!(error.contains("response is too large"), "{error}");
}

#[test]
fn accepts_exact_limit_and_preserves_binary_response() {
    let mut response = b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\n\r\n".to_vec();
    response.extend_from_slice(&vec![b'x'; 1048576]);
    assert_eq!(
        request_with_response(response, "text").unwrap().text.len(),
        1048576
    );
    let response = request_with_response(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n\x00\x01\xff\r\n0\r\n\r\n"
            .to_vec(),
        "base64",
    )
    .unwrap();
    assert_eq!(response.body_base64.as_deref(), Some("AAH/"));
    assert!(response.text.is_empty());
}

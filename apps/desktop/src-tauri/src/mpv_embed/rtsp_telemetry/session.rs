use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use super::{
    RtspTelemetrySnapshot, RtspTelemetryState,
    rtcp::{parse_rtcp_sender_report, receiver_report_packet},
    rtp::parse_rtp_header,
    rtsp::{RtspResponse, RtspUrl, parse_rtsp_url, read_rtsp_response, rtsp_track_url},
};

const RTSP_READ_TIMEOUT: Duration = Duration::from_millis(700);
const RTSP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const RTCP_RECEIVER_REPORT_INTERVAL: Duration = Duration::from_secs(1);
const RTSP_TELEMETRY_THREAD_PREFIX: &str = "openplayer-rtsp-telemetry";

pub(in crate::mpv_embed) struct RtspTelemetryHandle {
    state: Arc<Mutex<RtspTelemetryState>>,
    stop: Arc<AtomicBool>,
}

impl RtspTelemetryHandle {
    pub(in crate::mpv_embed) fn snapshot(&self) -> Option<RtspTelemetrySnapshot> {
        self.state.lock().ok()?.snapshot()
    }
}

impl Drop for RtspTelemetryHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub(in crate::mpv_embed) fn start_rtsp_receive_telemetry(
    rtsp_url: &str,
) -> Option<RtspTelemetryHandle> {
    let parsed = parse_rtsp_url(rtsp_url)?;
    let state = Arc::new(Mutex::new(RtspTelemetryState::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let thread_state = Arc::clone(&state);
    let thread_stop = Arc::clone(&stop);
    let name = format!("{RTSP_TELEMETRY_THREAD_PREFIX}-{}", parsed.host);
    let _ = thread::Builder::new().name(name).spawn(move || {
        let _ = run_rtsp_receive_telemetry(parsed, thread_state, thread_stop);
    });
    Some(RtspTelemetryHandle { state, stop })
}

fn run_rtsp_receive_telemetry(
    parsed: RtspUrl,
    state: Arc<Mutex<RtspTelemetryState>>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let stream = TcpStream::connect((parsed.host.as_str(), parsed.port))
        .map_err(|error| format!("rtsp telemetry connect failed: {error}"))?;
    stream
        .set_read_timeout(Some(RTSP_READ_TIMEOUT))
        .map_err(|error| format!("rtsp telemetry timeout setup failed: {error}"))?;
    stream
        .set_write_timeout(Some(RTSP_CONNECT_TIMEOUT))
        .map_err(|error| format!("rtsp telemetry write timeout setup failed: {error}"))?;

    let mut client = RtspTelemetryClient {
        stream,
        cseq: 1,
        session: String::new(),
        state,
        stop,
    };
    client.setup_and_play(&parsed.request_url)?;
    client.read_interleaved_loop();
    let _ = client.stream.shutdown(Shutdown::Both);
    Ok(())
}

struct RtspTelemetryClient {
    stream: TcpStream,
    cseq: u32,
    session: String,
    state: Arc<Mutex<RtspTelemetryState>>,
    stop: Arc<AtomicBool>,
}

impl RtspTelemetryClient {
    fn setup_and_play(&mut self, rtsp_url: &str) -> Result<(), String> {
        let _ = self.request("OPTIONS", rtsp_url, &[("User-Agent", "OpenPlayer")])?;
        let describe = self.request(
            "DESCRIBE",
            rtsp_url,
            &[("Accept", "application/sdp"), ("User-Agent", "OpenPlayer")],
        )?;
        let track_url = rtsp_track_url(rtsp_url, &describe.body);
        let setup = self.request(
            "SETUP",
            &track_url,
            &[
                ("Transport", "RTP/AVP/TCP;unicast;interleaved=0-1"),
                ("User-Agent", "OpenPlayer"),
            ],
        )?;
        self.session = setup
            .headers
            .get("session")
            .and_then(|value| value.split(';').next())
            .unwrap_or_default()
            .to_string();
        let _ = self.request("PLAY", rtsp_url, &[("User-Agent", "OpenPlayer")])?;
        Ok(())
    }

    fn request(
        &mut self,
        method: &str,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<RtspResponse, String> {
        let mut request = format!("{method} {url} RTSP/1.0\r\nCSeq: {}\r\n", self.cseq);
        self.cseq = self.cseq.saturating_add(1);
        if !self.session.is_empty() {
            request.push_str(&format!("Session: {}\r\n", self.session));
        }
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        self.stream
            .write_all(request.as_bytes())
            .map_err(|error| format!("rtsp telemetry request failed: {error}"))?;
        let response = read_rtsp_response(&mut self.stream)
            .map_err(|error| format!("rtsp telemetry response failed: {error}"))?;
        if !response.status_line.contains(" 200 ") {
            return Err(format!(
                "rtsp telemetry request rejected: {}",
                response.status_line
            ));
        }
        Ok(response)
    }

    fn read_interleaved_loop(&mut self) {
        let mut next_receiver_report = Instant::now();
        while !self.stop.load(Ordering::Relaxed) {
            if Instant::now() >= next_receiver_report {
                let _ = self.send_receiver_report();
                next_receiver_report = Instant::now() + RTCP_RECEIVER_REPORT_INTERVAL;
            }
            match self.read_interleaved_frame() {
                Ok(Some((channel, payload))) => {
                    if channel == 0 {
                        self.handle_rtp(&payload);
                    } else if channel == 1 {
                        self.handle_rtcp(&payload);
                    }
                }
                Ok(None) => {}
                Err(_) => break,
            }
        }
    }

    fn read_interleaved_frame(&mut self) -> std::io::Result<Option<(u8, Vec<u8>)>> {
        let mut marker = [0u8; 1];
        match self.stream.read_exact(&mut marker) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        }
        if marker[0] != b'$' {
            return Ok(None);
        }
        let mut header = [0u8; 3];
        self.stream.read_exact(&mut header)?;
        let channel = header[0];
        let length = u16::from_be_bytes([header[1], header[2]]) as usize;
        let mut payload = vec![0u8; length];
        self.stream.read_exact(&mut payload)?;
        Ok(Some((channel, payload)))
    }

    fn handle_rtp(&self, payload: &[u8]) {
        let Some(header) = parse_rtp_header(payload) else {
            return;
        };
        let receive_time_ns = unix_time_ns();
        if let Ok(mut state) = self.state.lock() {
            state.update_rtp(header.timestamp, receive_time_ns);
        }
    }

    fn handle_rtcp(&self, payload: &[u8]) {
        let Some(report) = parse_rtcp_sender_report(payload) else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            state.update_sender_report(report);
        }
    }

    fn send_receiver_report(&mut self) -> std::io::Result<()> {
        let report = receiver_report_packet(0x4f50_4c59);
        let mut frame = Vec::with_capacity(4 + report.len());
        frame.push(b'$');
        frame.push(1);
        frame.extend_from_slice(&(report.len() as u16).to_be_bytes());
        frame.extend_from_slice(&report);
        self.stream.write_all(&frame)
    }
}

fn unix_time_ns() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i128)
        .unwrap_or(0)
}

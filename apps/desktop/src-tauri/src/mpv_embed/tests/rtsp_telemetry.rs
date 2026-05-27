use super::*;

#[test]
fn rtsp_telemetry_parses_rtp_header_timestamp_and_ssrc() {
    let mut packet = [0u8; 12];
    packet[0] = 0x80;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&4321u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0x1234_5678u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0x90ab_cdefu32.to_be_bytes());

    let parsed = parse_rtp_header(&packet).unwrap();

    assert_eq!(parsed.sequence_number, 4321);
    assert_eq!(parsed.timestamp, 0x1234_5678);
    assert_eq!(parsed.ssrc, 0x90ab_cdef);
}

#[test]
fn rtsp_telemetry_parses_rtcp_sender_report() {
    let mut packet = vec![0u8; 28];
    packet[0] = 0x80;
    packet[1] = 200;
    packet[2..4].copy_from_slice(&6u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0x0102_0304u32.to_be_bytes());
    packet[8..12].copy_from_slice(&3_988_857_165u32.to_be_bytes());
    packet[12..16].copy_from_slice(&2_203_793_246u32.to_be_bytes());
    packet[16..20].copy_from_slice(&3_101_199_305u32.to_be_bytes());

    let parsed = parse_rtcp_sender_report(&packet).unwrap();

    assert_eq!(parsed.ssrc, 0x0102_0304);
    assert_eq!(parsed.ntp_seconds, 3_988_857_165);
    assert_eq!(parsed.ntp_fraction, 2_203_793_246);
    assert_eq!(parsed.rtp_timestamp, 3_101_199_305);
}

#[test]
fn rtsp_telemetry_parses_rtp_info_rtptime() {
    let info = "url=rtsp://127.0.0.1:8554/webm_rtsp_1/trackID=0;seq=64381;rtptime=3100344681";

    assert_eq!(rtsp_rtp_info_timestamp(info), Some(3_100_344_681));
}

#[test]
fn rtsp_telemetry_snapshot_uses_latest_rtp_and_sender_report() {
    let mut state = RtspTelemetryState::default();
    state.update_rtp(3_101_201_527, 1_779_868_366_185_000_000);
    state.update_sender_report(RtcpSenderReport {
        ssrc: 0,
        ntp_seconds: 3_988_857_165,
        ntp_fraction: 2_203_793_246,
        rtp_timestamp: 3_101_199_305,
    });

    let snapshot = state.snapshot().unwrap();

    assert_eq!(snapshot.source, RTSP_RECEIVE_LATENCY_SOURCE);
    assert!(snapshot.latency_ms > 500.0);
    assert!(snapshot.latency_ms < 700.0);
}

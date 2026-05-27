use super::*;

#[test]
fn rtp_timestamp_delta_handles_32_bit_wraparound() {
    assert_eq!(signed_rtp_delta_32(100, 40), 60);
    assert_eq!(signed_rtp_delta_32(40, 100), -60);
    assert_eq!(signed_rtp_delta_32(5, u32::MAX - 4), 10);
}

#[test]
fn rtcp_ntp_time_converts_to_unix_nanoseconds() {
    assert_eq!(rtcp_ntp_to_unix_ns(2_208_988_801, 0), Some(1_000_000_000));
    assert_eq!(
        rtcp_ntp_to_unix_ns(2_208_988_801, 2_147_483_648),
        Some(1_500_000_000)
    );
    assert_eq!(rtcp_ntp_to_unix_ns(2_208_988_799, 0), None);
}

#[test]
fn rtcp_rtp_map_computes_sender_time_and_latency() {
    let sender_time_ns = rtp_sender_time_ns(180_000, 90_000, 1_000_000_000, 90_000).unwrap();

    assert_eq!(sender_time_ns, 2_000_000_000);
    assert_eq!(
        transport_latency_ms(2_500_000_000, sender_time_ns),
        Some(500.0)
    );
}

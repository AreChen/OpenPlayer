#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mpv_embed) struct RtcpSenderReport {
    pub(in crate::mpv_embed) ssrc: u32,
    pub(in crate::mpv_embed) ntp_seconds: u32,
    pub(in crate::mpv_embed) ntp_fraction: u32,
    pub(in crate::mpv_embed) rtp_timestamp: u32,
}

pub(in crate::mpv_embed) fn parse_rtcp_sender_report(packet: &[u8]) -> Option<RtcpSenderReport> {
    let mut offset = 0usize;
    while offset + 4 <= packet.len() {
        let version = packet[offset] >> 6;
        let packet_type = packet[offset + 1];
        let words = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]) as usize;
        let length = words.checked_add(1)?.checked_mul(4)?;
        if version != 2 || length < 4 || offset + length > packet.len() {
            return None;
        }
        if packet_type == 200 && length >= 28 {
            return Some(RtcpSenderReport {
                ssrc: u32::from_be_bytes([
                    packet[offset + 4],
                    packet[offset + 5],
                    packet[offset + 6],
                    packet[offset + 7],
                ]),
                ntp_seconds: u32::from_be_bytes([
                    packet[offset + 8],
                    packet[offset + 9],
                    packet[offset + 10],
                    packet[offset + 11],
                ]),
                ntp_fraction: u32::from_be_bytes([
                    packet[offset + 12],
                    packet[offset + 13],
                    packet[offset + 14],
                    packet[offset + 15],
                ]),
                rtp_timestamp: u32::from_be_bytes([
                    packet[offset + 16],
                    packet[offset + 17],
                    packet[offset + 18],
                    packet[offset + 19],
                ]),
            });
        }
        offset += length;
    }
    None
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn receiver_report_packet(ssrc: u32) -> [u8; 8] {
    let mut packet = [0u8; 8];
    packet[0] = 0x80;
    packet[1] = 201;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
    packet
}

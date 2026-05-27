#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mpv_embed) struct RtpHeader {
    pub(in crate::mpv_embed) sequence_number: u16,
    pub(in crate::mpv_embed) timestamp: u32,
    pub(in crate::mpv_embed) ssrc: u32,
}

pub(in crate::mpv_embed) fn parse_rtp_header(packet: &[u8]) -> Option<RtpHeader> {
    if packet.len() < 12 || packet[0] >> 6 != 2 {
        return None;
    }
    Some(RtpHeader {
        sequence_number: u16::from_be_bytes([packet[2], packet[3]]),
        timestamp: u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]),
        ssrc: u32::from_be_bytes([packet[8], packet[9], packet[10], packet[11]]),
    })
}

//! Bounded RGBA transport for cooperating native presentation processes.
//!
//! This is a transport primitive, not an available player adapter. Control RPC
//! remains on stdio; pixels never enter JSON. The first implementation is Windows
//! only and deliberately uses CPU shared memory, not a claimed zero-copy GPU path.
use serde::{Deserialize, Serialize};
use std::io;

#[cfg(windows)]
pub mod windows;

pub const PROTOCOL: &str = "openplayer-present-rgba-v1";
pub const MAX_CAPACITY: u32 = 3840 * 2160 * 4;
pub const RESET: u32 = 1;
pub const REDRAW: u32 = 2;
pub const REPEAT: u32 = 4;
const NAME_PREFIX: &str = "Local\\OpenPlayer-Present-";
#[cfg(windows)]
const GLOBAL_SIZE: usize = 64;
#[cfg(any(windows, test))]
const FRAME_SIZE: usize = 64;
#[cfg(windows)]
const PAYLOAD_OFFSET: usize = GLOBAL_SIZE + FRAME_SIZE;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endpoint {
    pub protocol: String,
    pub mapping: String,
    pub capacity_bytes: u32,
    pub qpc_frequency: i64,
    pub producer_pid: u32,
}

impl Endpoint {
    pub fn validate(&self) -> io::Result<()> {
        let suffix = self.mapping.strip_prefix(NAME_PREFIX).unwrap_or("");
        if self.protocol != PROTOCOL
            || suffix.len() != 32
            || !suffix
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !(16 * 16 * 4..=MAX_CAPACITY).contains(&self.capacity_bytes)
            || self.qpc_frequency <= 0
            || self.producer_pid == 0
        {
            return Err(invalid("invalid presentation endpoint"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameMeta {
    pub sequence: u64,
    pub epoch: u64,
    /// QueryPerformanceCounter ticks, not mpv's process-relative clock.
    pub target_qpc: i64,
    pub duration_ns: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub flags: u32,
}

impl FrameMeta {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        if self.sequence == 0
            || self.epoch == 0
            || self.target_qpc < 0
            || self.duration_ns == 0
            || self.duration_ns > 10_000_000_000
            || !(16..=3840).contains(&self.width)
            || !(16..=2160).contains(&self.height)
            || self.stride != self.width * 4
            || self.flags & !(RESET | REDRAW | REPEAT) != 0
        {
            return Err(invalid("invalid presentation frame metadata"));
        }
        Ok(self.stride as usize * self.height as usize)
    }

    #[cfg(any(windows, test))]
    fn encode(&self, capacity: u32) -> io::Result<[u8; FRAME_SIZE]> {
        let size = self.payload_bytes()?;
        if size > capacity as usize {
            return Err(invalid("presentation frame exceeds capacity"));
        }
        let mut out = [0; FRAME_SIZE];
        out[0..8].copy_from_slice(&self.sequence.to_le_bytes());
        out[8..16].copy_from_slice(&self.epoch.to_le_bytes());
        out[16..24].copy_from_slice(&self.target_qpc.to_le_bytes());
        out[24..32].copy_from_slice(&self.duration_ns.to_le_bytes());
        out[32..36].copy_from_slice(&self.width.to_le_bytes());
        out[36..40].copy_from_slice(&self.height.to_le_bytes());
        out[40..44].copy_from_slice(&self.stride.to_le_bytes());
        out[44..48].copy_from_slice(&(size as u32).to_le_bytes());
        out[48..52].copy_from_slice(&self.flags.to_le_bytes());
        Ok(out)
    }

    #[cfg(any(windows, test))]
    fn decode(bytes: &[u8; FRAME_SIZE], capacity: u32) -> io::Result<Self> {
        let frame = Self {
            sequence: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            epoch: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            target_qpc: i64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            duration_ns: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            width: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
            height: u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
            stride: u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            flags: u32::from_le_bytes(bytes[48..52].try_into().unwrap()),
        };
        if frame.encode(capacity)? != *bytes {
            return Err(invalid("invalid presentation frame header"));
        }
        Ok(frame)
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn meta() -> FrameMeta {
        FrameMeta {
            sequence: 1,
            epoch: 1,
            target_qpc: 1,
            duration_ns: 33_333_333,
            width: 16,
            height: 16,
            stride: 64,
            flags: RESET,
        }
    }

    #[test]
    fn header_is_canonical_and_bounded() {
        let frame = meta();
        let bytes = frame.encode(1024).unwrap();
        assert_eq!(FrameMeta::decode(&bytes, 1024).unwrap(), frame);
        assert!(frame.encode(1023).is_err());
        for offset in [44, 52, 63] {
            let mut bad = bytes;
            bad[offset] ^= 1;
            assert!(FrameMeta::decode(&bad, 1024).is_err());
        }
        for bad in [
            FrameMeta {
                width: u32::MAX,
                ..frame
            },
            FrameMeta {
                stride: 65,
                ..frame
            },
            FrameMeta { flags: 8, ..frame },
            FrameMeta {
                sequence: 0,
                ..frame
            },
            FrameMeta { epoch: 0, ..frame },
            FrameMeta {
                duration_ns: 0,
                ..frame
            },
        ] {
            assert!(bad.encode(MAX_CAPACITY).is_err());
        }
    }

    #[test]
    fn endpoint_rejects_arbitrary_kernel_object_names() {
        let mut endpoint = Endpoint {
            protocol: PROTOCOL.into(),
            mapping: format!("{NAME_PREFIX}{}", "a".repeat(32)),
            capacity_bytes: 1024,
            qpc_frequency: 10_000_000,
            producer_pid: 1,
        };
        assert!(endpoint.validate().is_ok());
        for name in [
            "Global\\test",
            "Local\\test",
            "Local\\OpenPlayer-Present-../x",
        ] {
            endpoint.mapping = name.into();
            assert!(endpoint.validate().is_err());
        }
    }
}

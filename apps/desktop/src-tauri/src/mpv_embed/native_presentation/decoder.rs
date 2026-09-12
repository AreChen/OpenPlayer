use super::{
    cuda,
    render::Shared,
    transaction::{self, SavedOutput},
};
use libmpv2::Mpv;
use serde_json::Value;

pub(super) fn adapter(value: Option<&Value>) -> Result<Option<u64>, String> {
    value
        .map(|value| {
            value
                .as_str()
                .filter(|s| s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .and_then(|s| u64::from_str_radix(s, 16).ok())
                .filter(|luid| *luid != 0)
                .ok_or_else(|| "adapterLuid must be a nonzero 16-digit hexadecimal string".into())
        })
        .transpose()
}

fn select(
    requested: &str,
    luid: Option<u64>,
    lookup: impl FnOnce(u64) -> Result<i32, String>,
) -> Option<i32> {
    if requested == "no" {
        return None;
    }
    let luid = luid?;
    match lookup(luid) {
        Ok(device) => Some(device),
        Err(reason) => {
            eprintln!("OpenPlayer presentation uses software decoding: {reason}");
            None
        }
    }
}

pub(super) fn switch(
    mpv: &Mpv,
    saved: &SavedOutput,
    requested: &str,
    luid: Option<u64>,
    shared: &Shared,
) -> Result<(), String> {
    let software = SavedOutput {
        vo: "libmpv".into(),
        hwdec: "no".into(),
        cuda_device: saved.cuda_device.clone(),
    };
    let device = saved
        .cuda_device
        .as_ref()
        .and_then(|_| select(requested, luid, cuda::device_for_luid));
    if let Some(device) = device {
        let hardware = SavedOutput {
            hwdec: "nvdec-copy".into(),
            cuda_device: Some(device.to_string()),
            ..software.clone()
        };
        match transaction::switch(mpv, &hardware, shared) {
            Ok(()) => return Ok(()), // mpv falls back to software for unsupported codecs.
            Err(error) => {
                eprintln!("OpenPlayer hardware-copy switch failed; trying software: {error}")
            }
        }
    }
    transaction::switch(mpv, &software, shared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explicit_software_and_missing_adapter_never_probe_gpu() {
        assert_eq!(
            select("no", Some(1), |_| panic!(
                "software must not initialize CUDA"
            )),
            None
        );
        assert_eq!(
            select("auto-safe", None, |_| panic!("no default GPU")),
            None
        );
    }

    #[test]
    fn hardware_requires_an_exact_available_device() {
        assert_eq!(
            select("auto-safe", Some(0x2655b), |luid| {
                assert_eq!(luid, 0x2655b);
                Ok(1)
            }),
            Some(1)
        );
        assert_eq!(
            select("auto-safe", Some(1), |_| Err("unavailable".into())),
            None
        );
    }

    #[test]
    fn adapter_identity_is_not_an_ordinal_or_name() {
        assert_eq!(adapter(None).unwrap(), None);
        assert_eq!(
            adapter(Some(&json!("000000000002655B"))).unwrap(),
            Some(0x2655b)
        );
        for value in [
            json!(1),
            json!(null),
            json!("1"),
            json!("0000000000000000"),
            json!("000000000002655g"),
            json!("GPU 1"),
        ] {
            assert!(adapter(Some(&value)).is_err());
        }
    }
}

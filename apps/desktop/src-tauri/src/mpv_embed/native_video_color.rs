//! Fixed, host-owned color normalization shared by native video adapters.
use libmpv2::Mpv;
use serde_json::Value;

pub(crate) fn conversion_requested(value: Option<Value>) -> Result<bool, String> {
    match value.as_ref().and_then(Value::as_str) {
        None if value.is_none() => Ok(false),
        Some("none") => Ok(false),
        Some("sdr-bt709") => Ok(true),
        _ => Err("inputConversion must be none or sdr-bt709".into()),
    }
}

pub(super) fn sdr_filter(label: &str) -> String {
    // Apply Dolby Vision RPU while samples still have their original precision.
    format!(
        "@{label}:lavfi=[libplacebo=apply_dolbyvision=true:colorspace=bt709:color_primaries=bt709:color_trc=bt709:range=tv:format=yuv420p]"
    )
}

pub(super) fn is_sdr(matrix: &str, gamma: &str) -> bool {
    matrix == "bt.709" && matches!(gamma, "bt.1886" | "bt.709" | "srgb")
}

pub(super) struct Conversion {
    label: String,
}
impl Conversion {
    pub fn new(label: String) -> Self {
        Self { label }
    }

    pub fn install(&self, mpv: &Mpv) -> Result<(), String> {
        let chain = format!(
            "@{}-download:format=fmt=yuv420p10,{}",
            self.label,
            sdr_filter(&self.label)
        );
        mpv.command("vf", &["add", &chain])
            .map_err(|e| e.to_string())
    }

    pub fn remove(&self, mpv: &Mpv) -> Result<(), String> {
        let existing = super::native_video_filter::status::filters(mpv)?;
        let labels = [&self.label, &format!("{}-download", self.label)]
            .into_iter()
            .filter(|owned| existing.iter().any(|(label, _)| label == *owned))
            .map(|label| format!("@{label}"))
            .collect::<Vec<_>>();
        if !labels.is_empty() {
            mpv.command("vf", &["remove", &labels.join(",")])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn conversion_requires_an_explicit_supported_mode() {
        assert!(!conversion_requested(None).unwrap());
        assert!(!conversion_requested(Some(json!("none"))).unwrap());
        assert!(conversion_requested(Some(json!("sdr-bt709"))).unwrap());
        for value in [json!(null), json!(true), json!("auto"), json!({})] {
            assert!(conversion_requested(Some(value)).is_err());
        }
    }

    #[test]
    fn hdr_output_is_not_sdr() {
        assert!(!is_sdr("dolbyvision", "pq"));
        assert!(!is_sdr("bt.2020-ncl", "hlg"));
        assert!(!is_sdr("bt.709", "pq"));
        assert!(is_sdr("bt.709", "bt.1886"));
    }

    #[test]
    fn normalization_applies_dv_and_does_not_change_cadence_or_size() {
        let graph = sdr_filter("test-color");
        assert!(graph.contains("apply_dolbyvision=true"));
        assert!(graph.contains("color_trc=bt709:range=tv:format=yuv420p"));
        assert!(!graph.contains("fps="));
        assert!(!graph.contains(":w="));
    }
}

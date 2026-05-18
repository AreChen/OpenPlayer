#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpvSmokeReport {
    pub video_output: String,
    pub audio_output: String,
}

pub fn create_headless_probe() -> Result<MpvSmokeReport, String> {
    let _mpv = libmpv2::Mpv::with_initializer(|initializer| {
        initializer.set_property("vo", "null")?;
        initializer.set_property("ao", "null")?;
        Ok(())
    })
    .map_err(|error| error.to_string())?;

    Ok(MpvSmokeReport {
        video_output: "null".to_string(),
        audio_output: "null".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_libmpv_with_null_outputs() {
        let report = create_headless_probe().expect("libmpv should initialize with null outputs");

        assert_eq!(report.video_output, "null");
        assert_eq!(report.audio_output, "null");
    }
}

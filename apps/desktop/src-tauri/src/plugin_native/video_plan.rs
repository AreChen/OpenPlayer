use super::NativeModule;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VideoFormat {
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub pixel_format: String,
    pub color_space: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VideoStage {
    pub module_id: String,
    pub input: VideoFormat,
    pub output: VideoFormat,
    pub lookahead_frames: u32,
    pub max_output_frames: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VideoPlan {
    pub input: VideoFormat,
    pub stages: Vec<VideoStage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidatedVideoPlan {
    pub plan: VideoPlan,
    pub output: VideoFormat,
    pub lookahead_ms: f64,
    pub executable: bool,
}

fn validate_format(format: &VideoFormat) -> Result<(), String> {
    let rate = &format.frame_rate;
    if !(16..=8192).contains(&format.width)
        || !(16..=8192).contains(&format.height)
        || rate.numerator == 0
        || rate.denominator == 0
        || rate.numerator > 1_000_000
        || rate.denominator > 1_000_000
        || u64::from(rate.numerator) > 480 * u64::from(rate.denominator)
    {
        return Err("invalid native video dimensions or frame rate".into());
    }
    if !matches!(format.pixel_format.as_str(), "rgba8" | "rgba16f")
        || !matches!(
            format.color_space.as_str(),
            "srgb" | "linear-srgb" | "bt2020-pq"
        )
        || (format.color_space != "srgb" && format.pixel_format != "rgba16f")
    {
        return Err("invalid native video pixel/color contract".into());
    }
    Ok(())
}

// A control-plane validator, deliberately not an assertion of playback support.
pub(crate) fn validate_plan(
    mut plan: VideoPlan,
    modules: &[NativeModule],
) -> Result<ValidatedVideoPlan, String> {
    validate_format(&plan.input)?;
    normalize_rate(&mut plan.input.frame_rate);
    if plan.stages.is_empty() || plan.stages.len() > 8 {
        return Err("video plan requires 1..8 stages".into());
    }
    let mut previous = plan.input.clone();
    let mut lookahead_ms = 0.0;
    for stage in &mut plan.stages {
        if !modules.iter().any(|module| module.id == stage.module_id) {
            return Err("video stage references an undeclared native module".into());
        }
        validate_format(&stage.input)?;
        validate_format(&stage.output)?;
        normalize_rate(&mut stage.input.frame_rate);
        normalize_rate(&mut stage.output.frame_rate);
        if previous != stage.input {
            return Err("video stages have incompatible adjacent formats".into());
        }
        if stage.lookahead_frames > 8 || !(1..=8).contains(&stage.max_output_frames) {
            return Err("video stage exceeds bounded temporal buffers".into());
        }
        let input = &stage.input.frame_rate;
        let output = &stage.output.frame_rate;
        if u64::from(output.numerator) * u64::from(input.denominator)
            > u64::from(stage.max_output_frames)
                * u64::from(input.numerator)
                * u64::from(output.denominator)
        {
            return Err("frame rate increase exceeds maxOutputFrames".into());
        }
        lookahead_ms += 1000.0 * f64::from(stage.lookahead_frames) * f64::from(input.denominator)
            / f64::from(input.numerator);
        if lookahead_ms > 2000.0 {
            return Err("video plan lookahead exceeds two seconds".into());
        }
        previous = stage.output.clone();
    }
    Ok(ValidatedVideoPlan {
        plan,
        output: previous,
        lookahead_ms,
        executable: false,
    })
}

fn normalize_rate(rate: &mut FrameRate) {
    let (mut a, mut b) = (rate.numerator, rate.denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    rate.numerator /= a;
    rate.denominator /= a;
}

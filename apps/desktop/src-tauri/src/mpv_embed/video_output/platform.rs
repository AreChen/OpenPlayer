use std::fs;

use super::super::*;

#[cfg(target_os = "linux")]
pub(in crate::mpv_embed) fn platform_video_output_config() -> MpvVideoOutputConfig {
    let override_vo = std::env::var(OPENPLAYER_MPV_VO_ENV).ok();
    let override_gpu_context = std::env::var(OPENPLAYER_MPV_GPU_CONTEXT_ENV).ok();
    let override_hwdec = std::env::var(OPENPLAYER_MPV_HWDEC_ENV).ok();

    resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: override_vo.as_deref(),
        override_gpu_context: override_gpu_context.as_deref(),
        override_hwdec: override_hwdec.as_deref(),
        has_dri_render_node: has_linux_dri_render_node(),
        virtual_drm_driver: has_virtual_linux_drm_driver(),
    })
}

#[cfg(target_os = "macos")]
pub(in crate::mpv_embed) fn platform_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("libmpv".to_string()),
        gpu_context: None,
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
pub(in crate::mpv_embed) fn platform_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: None,
        gpu_context: None,
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(in crate::mpv_embed) fn resolve_linux_video_output_config(
    environment: LinuxVideoOutputEnvironment<'_>,
) -> MpvVideoOutputConfig {
    let override_vo = normalized_override(environment.override_vo);
    let override_gpu_context = normalized_override(environment.override_gpu_context);
    let override_hwdec = normalized_override(environment.override_hwdec);

    if let Some(vo) = override_vo {
        let vo_lower = vo.to_ascii_lowercase();
        let mut config = if vo_lower == "x11" {
            x11_software_video_output_config()
        } else {
            x11_gpu_video_output_config()
        };
        config.vo = Some(vo);
        if vo_lower != "gpu" && override_gpu_context.is_none() {
            config.gpu_context = None;
        }
        if let Some(gpu_context) = override_gpu_context {
            config.gpu_context = Some(gpu_context);
        }
        if let Some(hwdec) = override_hwdec {
            config.hwdec = hwdec;
        }
        return config;
    }

    let mut config = if environment.has_dri_render_node && !environment.virtual_drm_driver {
        x11_gpu_video_output_config()
    } else {
        x11_software_video_output_config()
    };

    if config.vo.as_deref() == Some("gpu")
        && let Some(gpu_context) = override_gpu_context
    {
        config.gpu_context = Some(gpu_context);
    }
    if let Some(hwdec) = override_hwdec {
        config.hwdec = hwdec;
    }

    config
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn x11_software_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("x11".to_string()),
        gpu_context: None,
        hwdec: "no".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn x11_gpu_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("gpu".to_string()),
        gpu_context: Some("x11egl".to_string()),
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn normalized_override(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn has_linux_dri_render_node() -> bool {
    let Ok(entries) = fs::read_dir("/dev/dri") else {
        return false;
    };

    entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("renderD"))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn has_virtual_linux_drm_driver() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return false;
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_link(entry.path().join("device/driver")).ok())
        .filter_map(|driver| {
            driver
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .any(|driver| is_virtual_linux_drm_driver(&driver))
}

pub(in crate::mpv_embed) fn is_virtual_linux_drm_driver(driver: &str) -> bool {
    let driver = driver.to_ascii_lowercase().replace('_', "-");

    matches!(
        driver.as_str(),
        "bochs" | "bochs-drm" | "cirrus" | "qxl" | "virtio-gpu"
    )
}

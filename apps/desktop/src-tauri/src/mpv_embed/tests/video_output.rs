use super::*;

#[test]
#[cfg(windows)]
fn encodes_win32_class_name_with_null_terminator() {
    let encoded = wide_null("STATIC");

    assert_eq!(encoded.last(), Some(&0));
    assert_eq!(encoded[..6], [83, 84, 65, 84, 73, 67]);
}

#[test]
fn prepares_numeric_locale_for_libmpv_initialization() {
    assert!(prepare_libmpv_numeric_locale().is_ok());
}

#[test]
fn linux_video_output_falls_back_to_x11_when_dri_render_node_is_missing() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: None,
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: false,
        virtual_drm_driver: false,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("x11".to_string()),
            gpu_context: None,
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn linux_video_output_falls_back_to_x11_for_virtual_drm_drivers() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: None,
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: true,
        virtual_drm_driver: true,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("x11".to_string()),
            gpu_context: None,
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn linux_video_output_uses_x11egl_when_dri_render_node_is_available() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: None,
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: true,
        virtual_drm_driver: false,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("gpu".to_string()),
            gpu_context: Some("x11egl".to_string()),
            hwdec: "auto-safe".to_string(),
        }
    );
}

#[test]
fn linux_video_output_allows_field_vo_override() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: Some("x11"),
        override_gpu_context: None,
        override_hwdec: None,
        has_dri_render_node: true,
        virtual_drm_driver: false,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("x11".to_string()),
            gpu_context: None,
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn linux_video_output_allows_gpu_context_and_hwdec_overrides() {
    let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: Some("gpu"),
        override_gpu_context: Some("x11"),
        override_hwdec: Some("no"),
        has_dri_render_node: false,
        virtual_drm_driver: true,
    });

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("gpu".to_string()),
            gpu_context: Some("x11".to_string()),
            hwdec: "no".to_string(),
        }
    );
}

#[test]
fn identifies_known_virtual_linux_drm_drivers() {
    assert!(is_virtual_linux_drm_driver("bochs-drm"));
    assert!(is_virtual_linux_drm_driver("QXL"));
    assert!(is_virtual_linux_drm_driver("virtio_gpu"));
    assert!(!is_virtual_linux_drm_driver("i915"));
    assert!(!is_virtual_linux_drm_driver("amdgpu"));
}

#[test]
fn forwards_mpv_video_diagnostic_log_messages() {
    assert!(is_mpv_video_diagnostic_log(
        "warn",
        "vo/gpu",
        "libEGL warning: DRI3 error: Could not get DRI3 device"
    ));
    assert!(is_mpv_video_diagnostic_log(
        "info",
        "cplayer",
        "VO: [x11] 1280x720 yuv420p"
    ));
    assert!(is_mpv_video_diagnostic_log(
        "v",
        "vd",
        "Trying hardware decoding via vaapi"
    ));
    assert!(!is_mpv_video_diagnostic_log(
        "info",
        "cplayer",
        "Playing: sample.mp4"
    ));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_video_output_uses_libmpv_render_api_vo() {
    let config = platform_video_output_config();

    assert_eq!(
        config,
        MpvVideoOutputConfig {
            vo: Some("libmpv".to_string()),
            gpu_context: None,
            hwdec: "auto-safe".to_string(),
        }
    );
}

#[test]
fn maps_x11_window_handles_to_mpv_wid_values() {
    let xlib = RawWindowHandle::Xlib(raw_window_handle::XlibWindowHandle::new(42));
    assert_eq!(mpv_wid_from_raw_window_handle(xlib).unwrap(), 42);

    let xcb_window = std::num::NonZeroU32::new(84).expect("fixture window id is non-zero");
    let xcb = RawWindowHandle::Xcb(raw_window_handle::XcbWindowHandle::new(xcb_window));
    assert_eq!(mpv_wid_from_raw_window_handle(xcb).unwrap(), 84);
}

#[test]
fn rejects_wayland_until_native_host_exists() {
    let surface = std::ptr::NonNull::dangling();
    let handle = RawWindowHandle::Wayland(raw_window_handle::WaylandWindowHandle::new(surface));

    assert_eq!(
        mpv_wid_from_raw_window_handle(handle).expect_err("Wayland does not support mpv wid"),
        "mpv embed playback currently supports Windows HWND and X11 window hosts; Wayland video host support is not implemented yet"
    );
}

#[test]
#[cfg(windows)]
fn reserves_web_controls_outside_native_video_host() {
    let rect = video_host_rect(1280, 720);

    assert_eq!(rect.x, 0);
    assert_eq!(rect.y, 0);
    assert_eq!(rect.width, 1280);
    assert_eq!(rect.height, 720);
}

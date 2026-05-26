use super::*;

pub(super) fn create_embed_player(hwnd: i64) -> Result<libmpv2::Mpv, String> {
    create_embed_player_with_log_subscription(hwnd, true)
}

pub(super) fn create_embed_player_without_logs(hwnd: i64) -> Result<libmpv2::Mpv, String> {
    create_embed_player_with_log_subscription(hwnd, false)
}

pub(super) fn create_embed_player_with_log_subscription(
    hwnd: i64,
    subscribe_logs: bool,
) -> Result<libmpv2::Mpv, String> {
    prepare_libmpv_numeric_locale()?;
    let video_output_config = platform_video_output_config();
    log_selected_mpv_video_output_config(&video_output_config);

    let mpv = libmpv2::Mpv::with_initializer(|initializer| {
        #[cfg(not(target_os = "macos"))]
        initializer.set_option("wid", hwnd)?;
        #[cfg(target_os = "macos")]
        let _ = hwnd;
        configure_native_video_output(&initializer, &video_output_config)?;
        #[cfg(target_os = "macos")]
        initializer.set_option("video-timing-offset", "0")?;
        initializer.set_option("input-default-bindings", false)?;
        initializer.set_option("input-vo-keyboard", false)?;
        initializer.set_option("keep-open", true)?;
        initializer.set_option("load-scripts", true)?;
        initializer.set_option("osc", false)?;
        Ok(())
    })
    .map_err(|error| format!("mpv embed init failed: {error}"))?;

    if subscribe_logs {
        request_mpv_log_messages(&mpv);
    }

    Ok(mpv)
}

#[cfg(target_os = "macos")]
pub(super) fn create_macos_render_context(
    mpv: &libmpv2::Mpv,
    host: &MpvVideoHost,
) -> Result<MacosMpvRenderContext, String> {
    unsafe {
        openplayer_mpv_gl_view_make_current(host.render_view_ptr());
    }

    let mut init_params = libmpv2_sys::mpv_opengl_init_params {
        get_proc_address: Some(macos_mpv_get_proc_address),
        get_proc_address_ctx: ptr::null_mut(),
    };
    let mut render_params = [
        libmpv2_sys::mpv_render_param {
            type_: libmpv2_sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            data: libmpv2_sys::MPV_RENDER_API_TYPE_OPENGL.as_ptr() as *mut c_void,
        },
        libmpv2_sys::mpv_render_param {
            type_: libmpv2_sys::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
            data: (&mut init_params as *mut libmpv2_sys::mpv_opengl_init_params).cast(),
        },
        libmpv2_sys::mpv_render_param {
            type_: 0,
            data: ptr::null_mut(),
        },
    ];
    let mut context: *mut libmpv2_sys::mpv_render_context = ptr::null_mut();
    let result = unsafe {
        libmpv2_sys::mpv_render_context_create(
            &mut context,
            mpv.ctx.as_ptr(),
            render_params.as_mut_ptr(),
        )
    };
    if result < 0 {
        return Err(format!(
            "mpv render context init failed: {}",
            mpv_error_message(result)
        ));
    }

    unsafe {
        openplayer_mpv_gl_view_set_render_context(host.render_view_ptr(), context.cast());
        libmpv2_sys::mpv_render_context_set_update_callback(
            context,
            Some(macos_mpv_render_update),
            host.render_view_ptr(),
        );
    }

    Ok(MacosMpvRenderContext {
        ctx: context as usize,
        view: host.render_view,
    })
}

#[cfg(target_os = "macos")]
impl Drop for MacosMpvRenderContext {
    fn drop(&mut self) {
        let context = self.ctx as *mut libmpv2_sys::mpv_render_context;
        unsafe {
            libmpv2_sys::mpv_render_context_set_update_callback(context, None, ptr::null_mut());
            openplayer_mpv_gl_view_set_render_context(self.view as *mut c_void, ptr::null_mut());
            libmpv2_sys::mpv_render_context_free(context);
        }
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn macos_mpv_get_proc_address(
    _ctx: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    unsafe { openplayer_mpv_gl_get_proc_address(name) }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn macos_mpv_render_update(ctx: *mut c_void) {
    unsafe {
        openplayer_mpv_gl_view_draw(ctx);
    }
}

pub(super) fn mpv_error_message(code: i32) -> String {
    let message = unsafe { libmpv2_sys::mpv_error_string(code) };
    if message.is_null() {
        return code.to_string();
    }

    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}

pub(super) fn configure_native_video_output(
    initializer: &libmpv2::MpvInitializer,
    config: &MpvVideoOutputConfig,
) -> libmpv2::Result<()> {
    apply_video_output_config(initializer, config)
}

pub(super) fn apply_video_output_config(
    initializer: &libmpv2::MpvInitializer,
    config: &MpvVideoOutputConfig,
) -> libmpv2::Result<()> {
    if let Some(vo) = config.vo.as_ref() {
        initializer.set_option("vo", vo.as_str())?;
    }
    if let Some(gpu_context) = config.gpu_context.as_ref() {
        initializer.set_option("gpu-context", gpu_context.as_str())?;
    }
    initializer.set_option("hwdec", config.hwdec.as_str())?;
    Ok(())
}

pub(super) fn request_mpv_log_messages(mpv: &libmpv2::Mpv) {
    let Ok(min_level) = CString::new("v") else {
        return;
    };
    let result =
        unsafe { libmpv2_sys::mpv_request_log_messages(mpv.ctx.as_ptr(), min_level.as_ptr()) };
    if result < 0 {
        eprintln!("OpenPlayer mpv log subscription failed: {result}");
    }
}

pub(super) fn log_selected_mpv_video_output_config(config: &MpvVideoOutputConfig) {
    eprintln!(
        "OpenPlayer mpv video output: vo={}, gpu-context={}, hwdec={}",
        config.vo.as_deref().unwrap_or("mpv-default"),
        config.gpu_context.as_deref().unwrap_or("mpv-default"),
        config.hwdec
    );
}

pub(super) fn log_mpv_video_diagnostic(prefix: &str, level: &str, text: &str) {
    if is_mpv_video_diagnostic_log(level, prefix, text) {
        eprintln!(
            "OpenPlayer mpv {level}/{prefix}: {}",
            text.trim_end_matches(['\r', '\n'])
        );
    }
}

pub(super) fn is_mpv_video_diagnostic_log(level: &str, prefix: &str, text: &str) -> bool {
    let level = level.to_ascii_lowercase();
    if matches!(level.as_str(), "fatal" | "error" | "warn") {
        return true;
    }

    let prefix = prefix.to_ascii_lowercase();
    if prefix.starts_with("vo") || matches!(prefix.as_str(), "vd" | "ffmpeg/video") {
        return true;
    }

    let text = text.to_ascii_lowercase();
    text.contains("vo:")
        || text.contains("[vo")
        || text.contains("gpu")
        || text.contains("egl")
        || text.contains("dri")
        || text.contains("vaapi")
        || text.contains("vdpau")
        || text.contains("hwdec")
}

#[cfg(target_os = "linux")]
pub(super) fn platform_video_output_config() -> MpvVideoOutputConfig {
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
pub(super) fn platform_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("libmpv".to_string()),
        gpu_context: None,
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
pub(super) fn platform_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: None,
        gpu_context: None,
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn resolve_linux_video_output_config(
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
pub(super) fn x11_software_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("x11".to_string()),
        gpu_context: None,
        hwdec: "no".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn x11_gpu_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("gpu".to_string()),
        gpu_context: Some("x11egl".to_string()),
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn normalized_override(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn has_linux_dri_render_node() -> bool {
    let Ok(entries) = fs::read_dir("/dev/dri") else {
        return false;
    };

    entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("renderD"))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn has_virtual_linux_drm_driver() -> bool {
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

pub(super) fn is_virtual_linux_drm_driver(driver: &str) -> bool {
    let driver = driver.to_ascii_lowercase().replace('_', "-");

    matches!(
        driver.as_str(),
        "bochs" | "bochs-drm" | "cirrus" | "qxl" | "virtio-gpu"
    )
}

#[cfg(unix)]
pub(super) fn prepare_libmpv_numeric_locale() -> Result<(), String> {
    let locale = std::ffi::CString::new("C")
        .map_err(|_| "failed to prepare LC_NUMERIC=C for libmpv".to_string())?;
    // SAFETY: libmpv requires the process C numeric locale to be "C" before
    // mpv_create(). We set only LC_NUMERIC immediately before initializing mpv.
    let result = unsafe { libc::setlocale(libc::LC_NUMERIC, locale.as_ptr()) };
    if result.is_null() {
        Err("failed to set LC_NUMERIC=C before libmpv initialization".to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub(super) fn prepare_libmpv_numeric_locale() -> Result<(), String> {
    Ok(())
}

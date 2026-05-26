use super::super::*;
use super::{
    locale::prepare_libmpv_numeric_locale,
    logging::{log_selected_mpv_video_output_config, request_mpv_log_messages},
    platform::platform_video_output_config,
};

pub(in crate::mpv_embed) fn create_embed_player(hwnd: i64) -> Result<libmpv2::Mpv, String> {
    create_embed_player_with_log_subscription(hwnd, true)
}

pub(in crate::mpv_embed) fn create_embed_player_without_logs(
    hwnd: i64,
) -> Result<libmpv2::Mpv, String> {
    create_embed_player_with_log_subscription(hwnd, false)
}

fn create_embed_player_with_log_subscription(
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

fn configure_native_video_output(
    initializer: &libmpv2::MpvInitializer,
    config: &MpvVideoOutputConfig,
) -> libmpv2::Result<()> {
    apply_video_output_config(initializer, config)
}

fn apply_video_output_config(
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

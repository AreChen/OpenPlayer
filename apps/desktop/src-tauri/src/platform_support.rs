use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSupport {
    os: &'static str,
    display_server: &'static str,
    mpv_embed_video: bool,
    native_shortcut_bridge: bool,
}

#[derive(Debug, Clone, Copy)]
struct PlatformEnvironment<'a> {
    os: &'static str,
    display: Option<&'a str>,
    wayland_display: Option<&'a str>,
    gdk_backend: Option<&'a str>,
}

#[tauri::command]
pub fn platform_support() -> PlatformSupport {
    PlatformSupport::current()
}

pub fn prepare_platform_runtime() {
    #[cfg(target_os = "linux")]
    prepare_linux_runtime();
}

impl PlatformSupport {
    fn current() -> Self {
        let display = std::env::var("DISPLAY").ok();
        let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let gdk_backend = std::env::var("GDK_BACKEND").ok();

        Self::for_environment(PlatformEnvironment {
            os: std::env::consts::OS,
            display: display.as_deref(),
            wayland_display: wayland_display.as_deref(),
            gdk_backend: gdk_backend.as_deref(),
        })
    }

    fn for_environment(environment: PlatformEnvironment<'_>) -> Self {
        let (display_server, mpv_embed_video) = native_video_support_for_environment(environment);

        Self {
            os: environment.os,
            display_server,
            mpv_embed_video,
            native_shortcut_bridge: environment.os == "windows",
        }
    }
}

fn native_video_support_for_environment(
    environment: PlatformEnvironment<'_>,
) -> (&'static str, bool) {
    match environment.os {
        "windows" => ("win32", true),
        "linux" => linux_video_support(environment),
        "macos" => ("appkit", true),
        _ => ("unknown", false),
    }
}

fn linux_video_support(environment: PlatformEnvironment<'_>) -> (&'static str, bool) {
    let gdk_backend = environment.gdk_backend.unwrap_or_default();
    let has_x11_backend = gdk_backend_allows(gdk_backend, "x11");
    let has_wayland_backend = gdk_backend_allows(gdk_backend, "wayland");
    let has_display = environment.display.is_some_and(|value| !value.is_empty());
    let has_wayland = has_wayland_backend
        || environment
            .wayland_display
            .is_some_and(|value| !value.is_empty());

    if has_display
        && (gdk_backend.is_empty() || has_x11_backend)
        && !gdk_backend_is_wayland_only(gdk_backend)
    {
        return ("x11", true);
    }

    if has_wayland {
        return ("wayland", false);
    }

    ("unknown", false)
}

fn gdk_backend_allows(backends: &str, backend: &str) -> bool {
    backends
        .split([',', ':', ';'])
        .map(str::trim)
        .any(|candidate| candidate.eq_ignore_ascii_case(backend))
}

fn gdk_backend_is_wayland_only(backends: &str) -> bool {
    gdk_backend_allows(backends, "wayland") && !gdk_backend_allows(backends, "x11")
}

#[cfg(target_os = "linux")]
fn prepare_linux_runtime() {
    let display = std::env::var("DISPLAY").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let gdk_backend = std::env::var("GDK_BACKEND").ok();
    let environment = PlatformEnvironment {
        os: "linux",
        display: display.as_deref(),
        wayland_display: wayland_display.as_deref(),
        gdk_backend: gdk_backend.as_deref(),
    };

    if should_default_linux_gdk_backend_to_x11(environment) {
        // SAFETY: this runs before Tauri initializes GTK/WebKit and before OpenPlayer
        // starts background threads, so changing process environment is bounded to
        // selecting the Linux UI backend for this process.
        unsafe {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn should_default_linux_gdk_backend_to_x11(environment: PlatformEnvironment<'_>) -> bool {
    environment.os == "linux"
        && environment.display.is_some_and(|value| !value.is_empty())
        && environment.gdk_backend.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_current_platform_name() {
        let support = PlatformSupport::current();

        assert_eq!(support.os, std::env::consts::OS);
    }

    #[test]
    fn native_shortcut_bridge_matches_current_platform_boundary() {
        let support = PlatformSupport::current();

        assert_eq!(support.native_shortcut_bridge, cfg!(windows));
    }

    #[test]
    fn linux_x11_session_supports_native_mpv_embedding() {
        let environment = PlatformEnvironment {
            os: "linux",
            display: Some(":0"),
            wayland_display: None,
            gdk_backend: None,
        };
        let support = PlatformSupport::for_environment(environment);

        assert_eq!(support.display_server, "x11");
        assert!(support.mpv_embed_video);
    }

    #[test]
    fn linux_wayland_only_session_rejects_native_mpv_embedding() {
        let environment = PlatformEnvironment {
            os: "linux",
            display: None,
            wayland_display: Some("wayland-0"),
            gdk_backend: None,
        };
        let support = PlatformSupport::for_environment(environment);

        assert_eq!(support.display_server, "wayland");
        assert!(!support.mpv_embed_video);
    }

    #[test]
    fn linux_explicit_wayland_backend_rejects_x11_embed_even_with_display() {
        let environment = PlatformEnvironment {
            os: "linux",
            display: Some(":0"),
            wayland_display: Some("wayland-0"),
            gdk_backend: Some("wayland"),
        };
        let support = PlatformSupport::for_environment(environment);

        assert_eq!(support.display_server, "wayland");
        assert!(!support.mpv_embed_video);
    }

    #[test]
    fn linux_default_runtime_prefers_x11_when_display_is_available() {
        let environment = PlatformEnvironment {
            os: "linux",
            display: Some(":0"),
            wayland_display: Some("wayland-0"),
            gdk_backend: None,
        };

        assert!(should_default_linux_gdk_backend_to_x11(environment));
    }

    #[test]
    fn linux_default_runtime_preserves_explicit_gdk_backend() {
        let environment = PlatformEnvironment {
            os: "linux",
            display: Some(":0"),
            wayland_display: Some("wayland-0"),
            gdk_backend: Some("wayland"),
        };

        assert!(!should_default_linux_gdk_backend_to_x11(environment));
    }

    #[test]
    fn macos_appkit_session_supports_native_mpv_embedding() {
        let environment = PlatformEnvironment {
            os: "macos",
            display: None,
            wayland_display: None,
            gdk_backend: None,
        };
        let support = PlatformSupport::for_environment(environment);

        assert_eq!(support.display_server, "appkit");
        assert!(support.mpv_embed_video);
    }
}

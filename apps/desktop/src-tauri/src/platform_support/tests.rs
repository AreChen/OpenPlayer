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

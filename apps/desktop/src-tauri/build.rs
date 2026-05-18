fn configure_mpv_linking() {
    if std::env::var_os("CARGO_FEATURE_MPV_SMOKE").is_none()
        && std::env::var_os("CARGO_FEATURE_MPV_EMBED").is_none()
        && std::env::var_os("CARGO_FEATURE_MPV_RENDER").is_none()
    {
        return;
    }

    println!("cargo:rerun-if-env-changed=OPENPLAYER_MPV_DIR");

    #[cfg(windows)]
    {
        let mpv_dir = std::env::var_os("OPENPLAYER_MPV_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let manifest_dir = std::path::PathBuf::from(
                    std::env::var("CARGO_MANIFEST_DIR")
                        .expect("CARGO_MANIFEST_DIR is set by Cargo"),
                );
                manifest_dir.join("../../../vendor/native/mpv/windows-x64")
            });
        let import_library = mpv_dir.join("libmpv.dll.a");
        let runtime_library = mpv_dir.join("libmpv-2.dll");

        if !import_library.exists() || !runtime_library.exists() {
            let missing = [
                (!import_library.exists()).then_some("libmpv.dll.a"),
                (!runtime_library.exists()).then_some("libmpv-2.dll"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");

            panic!(
                "mpv integration requires local ignored mpv artifacts at {} or OPENPLAYER_MPV_DIR; missing {}",
                mpv_dir.display(),
                missing
            );
        }

        println!("cargo:rustc-link-search=native={}", mpv_dir.display());
        println!("cargo:rerun-if-changed={}", import_library.display());
        println!("cargo:rerun-if-changed={}", runtime_library.display());
    }
}

fn main() {
    configure_mpv_linking();

    #[cfg(windows)]
    {
        let icon_path = std::path::PathBuf::from("icons").join("icon.ico");
        let windows = tauri_build::WindowsAttributes::new().window_icon_path(icon_path);
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("failed to run Tauri build script");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}

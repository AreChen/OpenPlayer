fn configure_mpv_linking() {
    if std::env::var_os("CARGO_FEATURE_MPV_SMOKE").is_none()
        && std::env::var_os("CARGO_FEATURE_MPV_EMBED").is_none()
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        println!("cargo:rerun-if-env-changed=PKG_CONFIG");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

        for link_search in unix_mpv_link_search_paths() {
            println!("cargo:rustc-link-search=native={}", link_search.display());
        }
    }
}

#[cfg(target_os = "macos")]
fn compile_macos_mpv_render_view() {
    if std::env::var_os("CARGO_FEATURE_MPV_EMBED").is_none() {
        return;
    }

    let mut build = cc::Build::new();
    build
        .file("src/macos_mpv_gl_view.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .flag("-Wno-deprecated-declarations");

    for include_path in pkg_config_include_paths("mpv") {
        build.include(include_path);
    }

    build.compile("openplayer_macos_mpv_gl_view");

    println!("cargo:rerun-if-changed=src/macos_mpv_gl_view.m");
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rustc-link-lib=framework=OpenGL");
}

#[cfg(not(target_os = "macos"))]
fn compile_macos_mpv_render_view() {}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unix_mpv_link_search_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if let Some(mpv_dir) = std::env::var_os("OPENPLAYER_MPV_DIR") {
        let mpv_dir = std::path::PathBuf::from(mpv_dir);
        if mpv_dir.join("lib").is_dir() {
            paths.push(mpv_dir.join("lib"));
        } else {
            paths.push(mpv_dir);
        }
    }

    if let Some(pkg_config_paths) = pkg_config_link_search_paths("mpv") {
        paths.extend(pkg_config_paths);
    }

    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|candidate| candidate == &path) {
            unique.push(path);
        }
    }

    unique
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn pkg_config_link_search_paths(package: &str) -> Option<Vec<std::path::PathBuf>> {
    let pkg_config = std::env::var_os("PKG_CONFIG").unwrap_or_else(|| "pkg-config".into());
    let output = std::process::Command::new(pkg_config)
        .args(["--libs-only-L", package])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(
        stdout
            .split_whitespace()
            .filter_map(|flag| flag.strip_prefix("-L"))
            .map(std::path::PathBuf::from)
            .collect(),
    )
}

#[cfg(target_os = "macos")]
fn pkg_config_include_paths(package: &str) -> Vec<std::path::PathBuf> {
    let pkg_config = std::env::var_os("PKG_CONFIG").unwrap_or_else(|| "pkg-config".into());
    let Ok(output) = std::process::Command::new(pkg_config)
        .args(["--cflags-only-I", package])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|flag| flag.strip_prefix("-I"))
        .map(std::path::PathBuf::from)
        .collect()
}

fn main() {
    configure_mpv_linking();
    compile_macos_mpv_render_view();

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

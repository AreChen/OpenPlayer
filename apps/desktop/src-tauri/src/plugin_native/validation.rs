use super::NativeModule;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs::File, io::Read, path::Path};

pub(crate) fn current_target() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

pub(crate) fn validate_modules(
    modules: &[NativeModule],
    permissions: &[String],
) -> Result<(), String> {
    if modules.is_empty() {
        return Ok(());
    }
    if modules.len() > 4 || !permissions.iter().any(|p| p == "native.process") {
        return Err("native modules require native.process and at most four modules".into());
    }
    let mut ids = HashSet::new();
    for module in modules {
        if let Some(adapter) = &module.video_adapter
            && (!matches!(adapter.as_str(), "vapoursynth-rgb-v1" | "present-rgba-v1")
                || !permissions.iter().any(|p| p == "native.video")
                || !["frames.open", "frames.status", "frames.close"]
                    .iter()
                    .all(|m| module.methods.iter().any(|v| v == m))
                || module.targets.keys().any(|p| p != "windows-x86_64"))
        {
            return Err(
                "video adapter requires native.video, Windows x64 and frames.open/status/close"
                    .into(),
            );
        }
        if !identifier(&module.id)
            || !ids.insert(&module.id)
            || module.protocol != "openplayer-native-v1"
        {
            return Err("invalid/duplicate native module id or unsupported protocol".into());
        }
        let mut methods = HashSet::new();
        if module.methods.is_empty()
            || module.methods.len() > 64
            || module
                .methods
                .iter()
                .any(|m| !identifier(m) || m.starts_with("host.") || !methods.insert(m))
        {
            return Err("native methods must be unique identifiers, excluding host.*".into());
        }
        if module.targets.is_empty() || module.targets.len() > 6 {
            return Err("invalid native targets".into());
        }
        for (platform, target) in &module.targets {
            if !matches!(
                platform.as_str(),
                "windows-x86_64"
                    | "windows-aarch64"
                    | "linux-x86_64"
                    | "linux-aarch64"
                    | "macos-x86_64"
                    | "macos-aarch64"
            ) {
                return Err("unsupported native target".into());
            }
            if target.entry.len() > 240
                || target.entry.contains(['\\', ':', '\0'])
                || target
                    .entry
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == "..")
                || (platform.starts_with("windows-") && !target.entry.ends_with(".exe"))
            {
                return Err("native entry must be a relative package executable path".into());
            }
            if target.sha256.len() != 64 || !target.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err("native executable requires a SHA256 digest".into());
            }
            if target.args.len() > 16
                || target
                    .args
                    .iter()
                    .any(|a| a.len() > 1024 || a.contains('\0'))
            {
                return Err("native fixed arguments exceed limits".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn verify_executable(path: &Path, expected: &str) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("native executable: {e}"))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 128 * 1024 * 1024 {
        return Err("native executable is too large".into());
    }
    let mut hash = Sha256::new();
    let mut file = file.take(128 * 1024 * 1024 + 1);
    let mut bytes_read = 0_u64;
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        bytes_read += count as u64;
        if bytes_read > 128 * 1024 * 1024 {
            return Err("native executable grew beyond size limit".into());
        }
        hash.update(&buffer[..count]);
    }
    if format!("{:x}", hash.finalize()) != expected.to_ascii_lowercase() {
        return Err("native executable integrity check failed; reinstall a trusted package".into());
    }
    Ok(())
}

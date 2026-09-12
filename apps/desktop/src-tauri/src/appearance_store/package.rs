use std::{
    fs,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use super::{
    MAX_PLUGIN_PACKAGE_FILES, MAX_PLUGIN_PACKAGE_UNCOMPRESSED_BYTES, PLUGIN_MANIFEST_FILE,
    manifest::validate_relative_plugin_entry,
};
pub(super) fn replace_directory_with_writer(
    target: &Path,
    staging: &Path,
    write: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create plugin install root: {error}"))?;
    }
    if staging.exists() {
        fs::remove_dir_all(staging)
            .map_err(|error| format!("failed to clear stale plugin staging directory: {error}"))?;
    }
    fs::create_dir_all(staging)
        .map_err(|error| format!("failed to create plugin staging directory: {error}"))?;

    if let Err(error) = write(staging) {
        let _ = fs::remove_dir_all(staging);
        return Err(error);
    }

    if target.exists() {
        fs::remove_dir_all(target)
            .map_err(|error| format!("failed to replace installed plugin directory: {error}"))?;
    }
    fs::rename(staging, target)
        .map_err(|error| format!("failed to finalize plugin installation: {error}"))
}

pub(super) fn copy_directory_contents(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create plugin install directory: {error}"))?;
    let source = source.canonicalize().map_err(|error| error.to_string())?;
    let target = target.canonicalize().map_err(|error| error.to_string())?;
    if target.starts_with(&source) {
        return Err("plugin source directory cannot contain its installation directory".into());
    }
    for entry in fs::read_dir(&source)
        .map_err(|error| format!("failed to read plugin directory: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read plugin directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect plugin directory entry: {error}"))?;
        if file_type.is_symlink() {
            return Err("plugin directories cannot contain symlinks".to_string());
        }

        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory_contents(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination)
                .map_err(|error| format!("failed to copy plugin file: {error}"))?;
        }
    }
    Ok(())
}

pub(super) fn validate_native_package(
    root: &Path,
    manifest: &super::types::PluginManifest,
) -> Result<(), String> {
    for module in &manifest.contributes.native_modules {
        for target in module.targets.values() {
            let file = resolve_plugin_package_file_path(&root.to_string_lossy(), &target.entry)?;
            crate::plugin_native::verify_executable(&file, &target.sha256)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                // ZIP metadata is not trusted to grant executable or special bits.
                fs::set_permissions(&file, fs::Permissions::from_mode(0o700))
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub(super) fn read_manifest_from_plugin_package(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open plugin package: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("invalid plugin package: {error}"))?;
    let mut manifest = archive
        .by_name(PLUGIN_MANIFEST_FILE)
        .map_err(|_| "plugin package must contain manifest.json at its root".to_string())?;
    if manifest.size() > 1024 * 1024 {
        return Err("plugin manifest is too large".to_string());
    }
    let mut json = String::new();
    manifest
        .read_to_string(&mut json)
        .map_err(|error| format!("failed to read plugin manifest from package: {error}"))?;
    Ok(json)
}

pub(super) fn extract_plugin_package(path: &Path, target: &Path) -> Result<(), String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open plugin package: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("invalid plugin package: {error}"))?;
    if archive.len() > MAX_PLUGIN_PACKAGE_FILES {
        return Err("plugin package contains too many files".to_string());
    }

    let mut total_uncompressed_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to read plugin package entry: {error}"))?;
        if entry.is_symlink() {
            return Err("plugin packages cannot contain symlinks".to_string());
        }
        total_uncompressed_size = total_uncompressed_size.saturating_add(entry.size());
        if total_uncompressed_size > MAX_PLUGIN_PACKAGE_UNCOMPRESSED_BYTES {
            return Err("plugin package is too large".to_string());
        }

        let Some(relative_path) = entry.enclosed_name() else {
            return Err("plugin package contains an unsafe path".to_string());
        };
        if relative_path.as_os_str().is_empty() {
            continue;
        }
        let output_path = target.join(relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("failed to create plugin package directory: {error}"))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create plugin package directory: {error}"))?;
        }
        let mut output = File::create(&output_path)
            .map_err(|error| format!("failed to extract plugin package file: {error}"))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("failed to write plugin package file: {error}"))?;
    }

    if !target.join(PLUGIN_MANIFEST_FILE).is_file() {
        return Err("plugin package must contain manifest.json at its root".to_string());
    }
    Ok(())
}

pub(super) fn remove_installed_plugin_directory(
    plugin_root: &Path,
    install_path: &Path,
) -> Result<(), String> {
    if !install_path.exists() {
        return Ok(());
    }
    let root = fs::canonicalize(plugin_root)
        .map_err(|error| format!("failed to resolve plugin root: {error}"))?;
    let target = fs::canonicalize(install_path)
        .map_err(|error| format!("failed to resolve plugin install directory: {error}"))?;
    if !target.starts_with(root) {
        return Err("plugin install path is outside the managed plugin directory".to_string());
    }
    // Native job termination can be followed by transient image-file deletion
    // denial on Windows. Retry that bounded cleanup, never a permanent lock.
    #[cfg(windows)]
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    #[cfg(windows)]
    loop {
        match fs::remove_dir_all(&target) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound && !target.exists() => {
                return Ok(());
            }
            Err(error)
                if matches!(error.raw_os_error(), Some(5 | 32 | 33))
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(error) => return Err(format!("failed to remove installed plugin files: {error}")),
        }
    }
    #[cfg(not(windows))]
    fs::remove_dir_all(target)
        .map_err(|error| format!("failed to remove installed plugin files: {error}"))
}

pub(super) fn resolve_plugin_runtime_script_path(
    install_path: &str,
    entry: &str,
) -> Result<PathBuf, String> {
    let script = resolve_plugin_package_file_path(install_path, entry)?;
    if !script.is_file() {
        return Err(format!("plugin runtime script is not a file: {entry}"));
    }
    Ok(script)
}

pub(super) fn resolve_plugin_package_file_path(
    install_path: &str,
    entry: &str,
) -> Result<PathBuf, String> {
    validate_relative_plugin_entry(entry)?;
    let install_root = PathBuf::from(install_path);
    let root = fs::canonicalize(&install_root)
        .map_err(|error| format!("failed to resolve plugin install path: {error}"))?;
    let candidate = install_root.join(entry);
    let file = fs::canonicalize(&candidate)
        .map_err(|error| format!("failed to resolve plugin package file: {error}"))?;
    if !file.starts_with(&root) {
        return Err("plugin package file is outside the installed plugin directory".to_string());
    }
    if !file.is_file() {
        return Err(format!("plugin package entry is not a file: {entry}"));
    }
    Ok(file)
}

#[cfg(test)]
mod copy_tests {
    use super::*;
    #[test]
    fn rejects_copying_a_plugin_into_its_own_descendant() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-copy-overlap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let result = copy_directory_contents(&directory, &directory.join("nested-install"));
        assert!(result.unwrap_err().contains("cannot contain"));
        fs::remove_dir_all(directory).unwrap();
    }
}

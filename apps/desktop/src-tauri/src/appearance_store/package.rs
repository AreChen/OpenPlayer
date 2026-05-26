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
    for entry in
        fs::read_dir(source).map_err(|error| format!("failed to read plugin directory: {error}"))?
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

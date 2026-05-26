use super::*;

#[test]
fn installs_manifest_file_into_managed_plugin_directory() {
    let (mut store, directory) = temp_store();
    let source_manifest = directory.join("source-subtitle-plugin.json");
    std::fs::write(&source_manifest, subtitle_plugin_json())
        .expect("source plugin manifest should be written");

    let state = store
        .import_plugin_manifest_path(&source_manifest)
        .expect("plugin manifest file should install");

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("installed plugin should be listed");
    let install_path = plugin
        .install_path
        .as_ref()
        .expect("installed plugin should expose install path");
    let installed_directory = PathBuf::from(install_path);

    assert_eq!(plugin.package_kind, "manifestFile");
    assert!(plugin.installed_at_ms.unwrap_or_default() > 0);
    assert!(installed_directory.ends_with("dev.openplayer.subtitle.styler"));
    assert!(installed_directory.join("manifest.json").exists());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn installs_plugin_directory_and_copies_package_assets() {
    let (mut store, directory) = temp_store();
    let source_directory = directory.join("subtitle-package");
    std::fs::create_dir_all(source_directory.join("assets"))
        .expect("source plugin package directory should be created");
    std::fs::write(
        source_directory.join("manifest.json"),
        subtitle_plugin_json(),
    )
    .expect("source plugin manifest should be written");
    std::fs::write(
        source_directory.join("assets").join("readme.txt"),
        "package asset",
    )
    .expect("source plugin asset should be written");

    let state = store
        .import_plugin_directory_path(&source_directory)
        .expect("plugin directory should install");

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("installed plugin should be listed");
    let install_path = plugin
        .install_path
        .as_ref()
        .expect("installed plugin should expose install path");
    let installed_directory = PathBuf::from(install_path);

    assert_eq!(plugin.package_kind, "directory");
    assert!(installed_directory.join("manifest.json").exists());
    assert!(
        installed_directory
            .join("assets")
            .join("readme.txt")
            .exists()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn installs_opplugin_package_and_extracts_assets() {
    let (mut store, directory) = temp_store();
    let package_path = directory.join("subtitle-styler.opplugin");
    write_opplugin_package(&package_path, subtitle_plugin_json());

    let state = store
        .import_plugin_package_path(&package_path)
        .expect("OpenPlayer plugin package should install");

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("installed plugin should be listed");
    let install_path = plugin
        .install_path
        .as_ref()
        .expect("installed plugin should expose install path");
    let installed_directory = PathBuf::from(install_path);

    assert_eq!(plugin.package_kind, "opplugin");
    assert!(installed_directory.join("manifest.json").exists());
    assert!(
        installed_directory
            .join("assets")
            .join("readme.txt")
            .exists()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn uninstalling_plugin_removes_state_settings_and_installed_files() {
    let (mut store, directory) = temp_store();
    let source_manifest = directory.join("subtitle-plugin.json");
    std::fs::write(&source_manifest, subtitle_plugin_json())
        .expect("source plugin manifest should be written");
    let installed = store
        .import_plugin_manifest_path(&source_manifest)
        .expect("plugin manifest file should install");
    let install_path = installed
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| plugin.install_path.clone())
        .expect("installed plugin should expose install path");
    store
        .set_plugin_setting(
            "dev.openplayer.subtitle.styler",
            "font-size",
            serde_json::json!(56),
        )
        .expect("valid plugin setting should persist");

    let state = store
        .uninstall_plugin("dev.openplayer.subtitle.styler")
        .expect("plugin should uninstall");
    assert!(
        !state
            .plugins
            .iter()
            .any(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
    );
    assert!(!PathBuf::from(&install_path).exists());

    let reinstalled = store
        .import_plugin_manifest_path(&source_manifest)
        .expect("plugin manifest file should reinstall");
    let font_size = reinstalled
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| {
            plugin
                .settings
                .iter()
                .find(|setting| setting.id == "font-size")
        })
        .map(|setting| setting.value.clone());
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(font_size, Some(serde_json::json!(42)));
}

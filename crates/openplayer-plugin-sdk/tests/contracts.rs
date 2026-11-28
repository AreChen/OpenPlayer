use openplayer_plugin_sdk::{
    API_VERSION, Action, HostAction, LocalizedText, Permission, PluginManifest,
};

fn subtitles() -> PluginManifest {
    PluginManifest {
        api_version: API_VERSION,
        id: "example.subtitles".into(),
        version: "1.0.0".into(),
        name: LocalizedText::new("Subtitle size", "字幕大小"),
        runtime: None,
        permissions: vec![Permission::SubtitleStyle],
        actions: vec![Action {
            id: "reset".into(),
            label: LocalizedText::new("Reset", "重置"),
            command: HostAction::SetSubtitleScale { scale: 1.0 },
        }],
    }
}

#[test]
fn round_trip_preserves_identity_labels_and_commands() {
    let original = subtitles();
    let json = original.to_json().unwrap();
    assert!(json.contains("\"subtitleStyle\""));
    assert!(json.contains("\"setSubtitleScale\""));
    let parsed = PluginManifest::from_json(json.as_bytes()).unwrap();
    assert_eq!(parsed, original);
    assert_eq!(parsed.name.get(true), "字幕大小");
}

#[test]
fn packages_must_use_a_supported_api_and_declared_capabilities() {
    let mut manifest = subtitles();
    manifest.api_version = 1;
    assert!(manifest.validate().is_err());
    manifest.api_version = API_VERSION;
    manifest.permissions.clear();
    assert!(manifest.validate().is_err());
    let json = subtitles()
        .to_json()
        .unwrap()
        .replace("setSubtitleScale", "unknown");
    assert!(PluginManifest::from_json(json.as_bytes()).is_err());
}

#[test]
fn ids_and_subtitle_values_are_validated_before_execution() {
    let mut manifest = subtitles();
    manifest.actions.push(manifest.actions[0].clone());
    assert!(manifest.validate().is_err());
    manifest.actions.pop();
    for scale in [0.1, f64::NAN, 4.01] {
        manifest.actions[0].command = HostAction::SetSubtitleScale { scale };
        assert!(manifest.validate().is_err());
    }
}

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginManifestError {
    #[error("plugin manifest JSON is invalid")]
    Json,
    #[error("plugin id must not be empty")]
    EmptyId,
    #[error("plugin name must not be empty")]
    EmptyName,
    #[error("plugin version must not be empty")]
    EmptyVersion,
    #[error("plugin id is invalid: {0}")]
    InvalidId(String),
    #[error("plugin version is invalid: {0}")]
    InvalidVersion(String),
    #[error("plugin description must not be empty")]
    EmptyDescription,
    #[error("plugin contribution id must not be empty: {0}")]
    EmptyContributionId(&'static str),
    #[error("plugin contribution title must not be empty: {0}")]
    EmptyContributionTitle(&'static str),
    #[error("plugin contribution id is invalid for {kind}: {id}")]
    InvalidContributionId { kind: &'static str, id: String },
    #[error("plugin contribution id is duplicated: {0}")]
    DuplicateContributionId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub entry: PluginEntry,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    #[serde(default)]
    pub contributes: PluginContributions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginEntry {
    BuiltIn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginPermission {
    #[serde(rename = "metadata.read")]
    MetadataRead,
    #[serde(rename = "subtitle.search")]
    SubtitleSearch,
    #[serde(rename = "settings.read")]
    SettingsRead,
    #[serde(rename = "settings.write")]
    SettingsWrite,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginContributions {
    #[serde(default)]
    pub commands: Vec<PluginContribution>,
    #[serde(default)]
    pub settings_pages: Vec<PluginContribution>,
    #[serde(default)]
    pub metadata_providers: Vec<PluginContribution>,
    #[serde(default)]
    pub subtitle_sources: Vec<PluginContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginContribution {
    pub id: String,
    pub title: String,
}

pub fn parse_plugin_manifest_json(json: &str) -> Result<PluginManifest, PluginManifestError> {
    let manifest = serde_json::from_str(json).map_err(|_| PluginManifestError::Json)?;
    validate_plugin_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_plugin_manifest(manifest: &PluginManifest) -> Result<(), PluginManifestError> {
    if manifest.id.trim().is_empty() {
        return Err(PluginManifestError::EmptyId);
    }
    if manifest.name.trim().is_empty() {
        return Err(PluginManifestError::EmptyName);
    }
    if manifest.version.trim().is_empty() {
        return Err(PluginManifestError::EmptyVersion);
    }
    if !is_dotted_lowercase_identifier(&manifest.id) {
        return Err(PluginManifestError::InvalidId(manifest.id.clone()));
    }
    if !is_simple_semver(&manifest.version) {
        return Err(PluginManifestError::InvalidVersion(
            manifest.version.clone(),
        ));
    }
    if manifest
        .description
        .as_ref()
        .is_some_and(|description| description.trim().is_empty())
    {
        return Err(PluginManifestError::EmptyDescription);
    }

    let mut ids = HashSet::new();
    validate_contributions("commands", &manifest.contributes.commands, &mut ids)?;
    validate_contributions(
        "settingsPages",
        &manifest.contributes.settings_pages,
        &mut ids,
    )?;
    validate_contributions(
        "metadataProviders",
        &manifest.contributes.metadata_providers,
        &mut ids,
    )?;
    validate_contributions(
        "subtitleSources",
        &manifest.contributes.subtitle_sources,
        &mut ids,
    )?;

    Ok(())
}

fn validate_contributions(
    kind: &'static str,
    contributions: &[PluginContribution],
    ids: &mut HashSet<String>,
) -> Result<(), PluginManifestError> {
    for contribution in contributions {
        if contribution.id.trim().is_empty() {
            return Err(PluginManifestError::EmptyContributionId(kind));
        }
        if contribution.title.trim().is_empty() {
            return Err(PluginManifestError::EmptyContributionTitle(kind));
        }
        if !is_dotted_lowercase_identifier(&contribution.id) {
            return Err(PluginManifestError::InvalidContributionId {
                kind,
                id: contribution.id.clone(),
            });
        }
        if !ids.insert(contribution.id.clone()) {
            return Err(PluginManifestError::DuplicateContributionId(
                contribution.id.clone(),
            ));
        }
    }

    Ok(())
}

fn is_dotted_lowercase_identifier(value: &str) -> bool {
    let mut segment_count = 0;
    for segment in value.split('.') {
        segment_count += 1;

        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_lowercase() {
            return false;
        }
        if !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
            return false;
        }
    }

    segment_count >= 2
}

fn is_simple_semver(value: &str) -> bool {
    let mut part_count = 0;
    for part in value.split('.') {
        part_count += 1;
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
    }

    part_count == 3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_json() -> &'static str {
        r#"
        {
          "id": "dev.openplayer.metadata",
          "name": "Metadata Helper",
          "version": "1.2.3",
          "description": "Adds metadata lookup commands.",
          "entry": "builtIn",
          "permissions": ["metadata.read", "settings.read"],
          "contributes": {
            "commands": [
              { "id": "dev.openplayer.metadata.refresh", "title": "Refresh metadata" }
            ],
            "settingsPages": [
              { "id": "dev.openplayer.metadata.settings", "title": "Metadata settings" }
            ],
            "metadataProviders": [
              { "id": "dev.openplayer.metadata.provider", "title": "Local metadata" }
            ],
            "subtitleSources": [
              { "id": "dev.openplayer.metadata.subtitles", "title": "Subtitle search" }
            ]
          }
        }
        "#
    }

    #[test]
    fn parses_valid_manifest_json() {
        let manifest = parse_plugin_manifest_json(valid_manifest_json()).unwrap();

        assert_eq!(manifest.id, "dev.openplayer.metadata");
        assert_eq!(manifest.name, "Metadata Helper");
        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(
            manifest.description.as_deref(),
            Some("Adds metadata lookup commands.")
        );
        assert_eq!(manifest.entry, PluginEntry::BuiltIn);
        assert_eq!(
            manifest.permissions,
            vec![
                PluginPermission::MetadataRead,
                PluginPermission::SettingsRead
            ]
        );
        assert_eq!(
            manifest.contributes.commands[0].id,
            "dev.openplayer.metadata.refresh"
        );
    }

    #[test]
    fn malformed_json_returns_json_error() {
        assert_eq!(
            parse_plugin_manifest_json("{not valid json"),
            Err(PluginManifestError::Json)
        );
    }

    #[test]
    fn rejects_unknown_manifest_fields() {
        let json = r#"
        {
          "id": "dev.openplayer.metadata",
          "name": "Metadata Helper",
          "version": "1.2.3",
          "entry": "builtIn",
          "unexpected": true
        }
        "#;

        assert_eq!(
            parse_plugin_manifest_json(json),
            Err(PluginManifestError::Json)
        );
    }

    #[test]
    fn rejects_unknown_permissions() {
        let json = valid_manifest_json().replace("metadata.read", "filesystem.read");

        assert_eq!(
            parse_plugin_manifest_json(&json),
            Err(PluginManifestError::Json)
        );
    }

    #[test]
    fn rejects_empty_required_fields() {
        let manifest = parse_plugin_manifest_json(valid_manifest_json()).unwrap();

        let mut empty_id = manifest.clone();
        empty_id.id = " ".to_string();
        assert_eq!(
            validate_plugin_manifest(&empty_id),
            Err(PluginManifestError::EmptyId)
        );

        let mut empty_name = manifest.clone();
        empty_name.name = "\t".to_string();
        assert_eq!(
            validate_plugin_manifest(&empty_name),
            Err(PluginManifestError::EmptyName)
        );

        let mut empty_version = manifest.clone();
        empty_version.version.clear();
        assert_eq!(
            validate_plugin_manifest(&empty_version),
            Err(PluginManifestError::EmptyVersion)
        );
    }

    #[test]
    fn validates_identifier_and_semver_format() {
        let manifest = parse_plugin_manifest_json(valid_manifest_json()).unwrap();

        let mut invalid_id = manifest.clone();
        invalid_id.id = "OpenPlayer.Metadata".to_string();
        assert_eq!(
            validate_plugin_manifest(&invalid_id),
            Err(PluginManifestError::InvalidId(
                "OpenPlayer.Metadata".to_string()
            ))
        );

        let mut single_segment_id = manifest.clone();
        single_segment_id.id = "metadata".to_string();
        assert_eq!(
            validate_plugin_manifest(&single_segment_id),
            Err(PluginManifestError::InvalidId("metadata".to_string()))
        );

        let mut invalid_version = manifest.clone();
        invalid_version.version = "1.2".to_string();
        assert_eq!(
            validate_plugin_manifest(&invalid_version),
            Err(PluginManifestError::InvalidVersion("1.2".to_string()))
        );
    }

    #[test]
    fn rejects_blank_description_when_present() {
        let mut manifest = parse_plugin_manifest_json(valid_manifest_json()).unwrap();
        manifest.description = Some("  ".to_string());

        assert_eq!(
            validate_plugin_manifest(&manifest),
            Err(PluginManifestError::EmptyDescription)
        );
    }

    #[test]
    fn validates_contribution_ids_titles_and_duplicates() {
        let manifest = parse_plugin_manifest_json(valid_manifest_json()).unwrap();

        let mut empty_id = manifest.clone();
        empty_id.contributes.commands[0].id.clear();
        assert_eq!(
            validate_plugin_manifest(&empty_id),
            Err(PluginManifestError::EmptyContributionId("commands"))
        );

        let mut empty_title = manifest.clone();
        empty_title.contributes.settings_pages[0].title = " ".to_string();
        assert_eq!(
            validate_plugin_manifest(&empty_title),
            Err(PluginManifestError::EmptyContributionTitle("settingsPages"))
        );

        let mut invalid_id = manifest.clone();
        invalid_id.contributes.metadata_providers[0].id = "Metadata Provider".to_string();
        assert_eq!(
            validate_plugin_manifest(&invalid_id),
            Err(PluginManifestError::InvalidContributionId {
                kind: "metadataProviders",
                id: "Metadata Provider".to_string(),
            })
        );

        let mut duplicate = manifest.clone();
        duplicate.contributes.subtitle_sources[0].id =
            "dev.openplayer.metadata.refresh".to_string();
        assert_eq!(
            validate_plugin_manifest(&duplicate),
            Err(PluginManifestError::DuplicateContributionId(
                "dev.openplayer.metadata.refresh".to_string()
            ))
        );
    }
}

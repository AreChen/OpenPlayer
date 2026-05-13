use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: PluginEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginEntry {
    BuiltIn,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginManifestError {
    #[error("plugin id must not be empty")]
    EmptyId,
    #[error("plugin name must not be empty")]
    EmptyName,
    #[error("plugin version must not be empty")]
    EmptyVersion,
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_builtin_plugin_manifest() {
        let manifest = PluginManifest {
            id: "openplayer.core".to_string(),
            name: "OpenPlayer Core".to_string(),
            version: "0.1.0".to_string(),
            entry: PluginEntry::BuiltIn,
        };

        assert_eq!(validate_plugin_manifest(&manifest), Ok(()));
    }

    #[test]
    fn rejects_empty_plugin_id() {
        let manifest = PluginManifest {
            id: " ".to_string(),
            name: "OpenPlayer Core".to_string(),
            version: "0.1.0".to_string(),
            entry: PluginEntry::BuiltIn,
        };

        assert_eq!(
            validate_plugin_manifest(&manifest),
            Err(PluginManifestError::EmptyId)
        );
    }
}

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub tokens: ThemeTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeTokens {
    pub surface: String,
    pub surface_elevated: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent: String,
    pub border: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ThemeManifestError {
    #[error("theme id must not be empty")]
    EmptyId,
    #[error("theme name must not be empty")]
    EmptyName,
}

pub fn studio_dark_manifest() -> ThemeManifest {
    ThemeManifest {
        id: "studio-dark".to_string(),
        name: "Studio Dark".to_string(),
        version: "0.1.0".to_string(),
        tokens: ThemeTokens {
            surface: "#080A0F".to_string(),
            surface_elevated: "#111722".to_string(),
            text_primary: "#E9EEF8".to_string(),
            text_muted: "#AEB9CC".to_string(),
            accent: "#5B8CFF".to_string(),
            border: "rgba(255,255,255,0.10)".to_string(),
        },
    }
}

pub fn validate_theme_manifest(manifest: &ThemeManifest) -> Result<(), ThemeManifestError> {
    if manifest.id.trim().is_empty() {
        return Err(ThemeManifestError::EmptyId);
    }
    if manifest.name.trim().is_empty() {
        return Err(ThemeManifestError::EmptyName);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_dark_manifest_is_valid() {
        let manifest = studio_dark_manifest();

        assert_eq!(manifest.id, "studio-dark");
        assert_eq!(manifest.name, "Studio Dark");
        assert_eq!(validate_theme_manifest(&manifest), Ok(()));
    }
}

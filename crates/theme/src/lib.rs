use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ThemeManifestError {
    #[error("theme manifest JSON is invalid")]
    Json,
    #[error("theme id must not be empty")]
    EmptyId,
    #[error("theme name must not be empty")]
    EmptyName,
    #[error("theme version must not be empty")]
    EmptyVersion,
    #[error("theme id is invalid: {0}")]
    InvalidId(String),
    #[error("theme version is invalid: {0}")]
    InvalidVersion(String),
    #[error("theme color token {token} is invalid: {value}")]
    InvalidColorToken { token: &'static str, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub tokens: ThemeTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTokens {
    pub surface: String,
    pub panel: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub danger: String,
    pub border: String,
    pub radius: ThemeRadius,
    pub density: ThemeDensity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeRadius {
    None,
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeDensity {
    Compact,
    Comfortable,
    Spacious,
}

pub fn studio_dark_manifest() -> ThemeManifest {
    ThemeManifest {
        id: "studio-dark".to_string(),
        name: "Studio Dark".to_string(),
        version: "1.0.0".to_string(),
        tokens: ThemeTokens {
            surface: "#050607".to_string(),
            panel: "rgba(8, 10, 12, 0.88)".to_string(),
            text: "#ece7dd".to_string(),
            muted: "#b9b0a3".to_string(),
            accent: "#caa05d".to_string(),
            danger: "#d78372".to_string(),
            border: "rgba(236, 231, 221, 0.12)".to_string(),
            radius: ThemeRadius::Medium,
            density: ThemeDensity::Comfortable,
        },
    }
}

pub fn parse_theme_manifest_json(json: &str) -> Result<ThemeManifest, ThemeManifestError> {
    let manifest: ThemeManifest =
        serde_json::from_str(json).map_err(|_| ThemeManifestError::Json)?;
    validate_theme_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_theme_manifest(manifest: &ThemeManifest) -> Result<(), ThemeManifestError> {
    if manifest.id.trim().is_empty() {
        return Err(ThemeManifestError::EmptyId);
    }
    if manifest.name.trim().is_empty() {
        return Err(ThemeManifestError::EmptyName);
    }
    if manifest.version.trim().is_empty() {
        return Err(ThemeManifestError::EmptyVersion);
    }
    if !is_valid_theme_id(&manifest.id) {
        return Err(ThemeManifestError::InvalidId(manifest.id.clone()));
    }
    if !is_simple_semver(&manifest.version) {
        return Err(ThemeManifestError::InvalidVersion(manifest.version.clone()));
    }

    validate_color_token("surface", &manifest.tokens.surface)?;
    validate_color_token("panel", &manifest.tokens.panel)?;
    validate_color_token("text", &manifest.tokens.text)?;
    validate_color_token("muted", &manifest.tokens.muted)?;
    validate_color_token("accent", &manifest.tokens.accent)?;
    validate_color_token("danger", &manifest.tokens.danger)?;
    validate_color_token("border", &manifest.tokens.border)?;

    Ok(())
}

fn is_valid_theme_id(id: &str) -> bool {
    let mut saw_segment = false;
    for segment in id.split('.') {
        saw_segment = true;
        if segment.is_empty() || !is_valid_id_segment(segment) {
            return false;
        }
    }
    saw_segment
}

fn is_valid_id_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_simple_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn validate_color_token(token: &'static str, value: &str) -> Result<(), ThemeManifestError> {
    if is_valid_hex_color(value) || is_valid_rgba_color(value) {
        return Ok(());
    }

    Err(ThemeManifestError::InvalidColorToken {
        token,
        value: value.to_string(),
    })
}

fn is_valid_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_rgba_color(value: &str) -> bool {
    let Some(contents) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };

    let parts: Vec<&str> = contents.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return false;
    }

    let rgb_valid = parts[..3]
        .iter()
        .all(|part| part.parse::<u8>().is_ok_and(|_| is_plain_integer(part)));
    let alpha_valid = parts[3]
        .parse::<f64>()
        .is_ok_and(|alpha| alpha.is_finite() && (0.0..=1.0).contains(&alpha));

    rgb_valid && alpha_valid
}

fn is_plain_integer(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_json() -> &'static str {
        r##"
        {
          "id": "studio-dark",
          "name": "Studio Dark",
          "version": "1.0.0",
          "tokens": {
            "surface": "#050607",
            "panel": "rgba(8, 10, 12, 0.88)",
            "text": "#ece7dd",
            "muted": "#b9b0a3",
            "accent": "#caa05d",
            "danger": "#d78372",
            "border": "rgba(236, 231, 221, 0.12)",
            "radius": "medium",
            "density": "comfortable"
          }
        }
        "##
    }

    #[test]
    fn parses_valid_theme_manifest_json() {
        let manifest = parse_theme_manifest_json(valid_manifest_json()).unwrap();

        assert_eq!(manifest.id, "studio-dark");
        assert_eq!(manifest.name, "Studio Dark");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.tokens.surface, "#050607");
        assert_eq!(manifest.tokens.panel, "rgba(8, 10, 12, 0.88)");
        assert_eq!(manifest.tokens.radius, ThemeRadius::Medium);
        assert_eq!(manifest.tokens.density, ThemeDensity::Comfortable);
    }

    #[test]
    fn malformed_json_returns_json_error() {
        assert_eq!(
            parse_theme_manifest_json("{not valid json"),
            Err(ThemeManifestError::Json)
        );
    }

    #[test]
    fn studio_dark_manifest_is_valid() {
        let manifest = studio_dark_manifest();

        validate_theme_manifest(&manifest).unwrap();
        assert_eq!(manifest.id, "studio-dark");
        assert_eq!(manifest.tokens.radius, ThemeRadius::Medium);
        assert_eq!(manifest.tokens.density, ThemeDensity::Comfortable);
    }

    #[test]
    fn rejects_unknown_manifest_and_token_fields() {
        let manifest_json = valid_manifest_json().replace(
            "\"tokens\": {",
            "\"unexpected\": true,\n          \"tokens\": {",
        );
        assert_eq!(
            parse_theme_manifest_json(&manifest_json),
            Err(ThemeManifestError::Json)
        );

        let token_json = valid_manifest_json().replace(
            "\"density\": \"comfortable\"",
            "\"density\": \"comfortable\",\n            \"shadow\": \"large\"",
        );
        assert_eq!(
            parse_theme_manifest_json(&token_json),
            Err(ThemeManifestError::Json)
        );

        let radius_json =
            valid_manifest_json().replace("\"radius\": \"medium\"", "\"radius\": \"round\"");
        assert_eq!(
            parse_theme_manifest_json(&radius_json),
            Err(ThemeManifestError::Json)
        );

        let density_json =
            valid_manifest_json().replace("\"density\": \"comfortable\"", "\"density\": \"dense\"");
        assert_eq!(
            parse_theme_manifest_json(&density_json),
            Err(ThemeManifestError::Json)
        );
    }

    #[test]
    fn rejects_empty_required_fields() {
        let manifest = parse_theme_manifest_json(valid_manifest_json()).unwrap();

        let mut empty_id = manifest.clone();
        empty_id.id = " ".to_string();
        assert_eq!(
            validate_theme_manifest(&empty_id),
            Err(ThemeManifestError::EmptyId)
        );

        let mut empty_name = manifest.clone();
        empty_name.name = "\n".to_string();
        assert_eq!(
            validate_theme_manifest(&empty_name),
            Err(ThemeManifestError::EmptyName)
        );

        let mut empty_version = manifest.clone();
        empty_version.version.clear();
        assert_eq!(
            validate_theme_manifest(&empty_version),
            Err(ThemeManifestError::EmptyVersion)
        );
    }

    #[test]
    fn validates_theme_identifier_and_semver_format() {
        let manifest = parse_theme_manifest_json(valid_manifest_json()).unwrap();

        let mut invalid_id = manifest.clone();
        invalid_id.id = "Studio Dark".to_string();
        assert_eq!(
            validate_theme_manifest(&invalid_id),
            Err(ThemeManifestError::InvalidId("Studio Dark".to_string()))
        );

        let mut dotted_id = manifest.clone();
        dotted_id.id = "dev.openplayer.studio-dark".to_string();
        validate_theme_manifest(&dotted_id).unwrap();

        let mut invalid_version = manifest.clone();
        invalid_version.version = "1.0".to_string();
        assert_eq!(
            validate_theme_manifest(&invalid_version),
            Err(ThemeManifestError::InvalidVersion("1.0".to_string()))
        );
    }

    #[test]
    fn validates_color_token_formats() {
        let manifest = parse_theme_manifest_json(valid_manifest_json()).unwrap();

        let mut short_hex = manifest.clone();
        short_hex.tokens.accent = "#abc".to_string();
        validate_theme_manifest(&short_hex).unwrap();

        let mut bad_hex = manifest.clone();
        bad_hex.tokens.text = "#12xx56".to_string();
        assert_eq!(
            validate_theme_manifest(&bad_hex),
            Err(ThemeManifestError::InvalidColorToken {
                token: "text",
                value: "#12xx56".to_string(),
            })
        );

        let mut bad_rgba = manifest.clone();
        bad_rgba.tokens.panel = "rgba(1, 2, 300, 0.5)".to_string();
        assert_eq!(
            validate_theme_manifest(&bad_rgba),
            Err(ThemeManifestError::InvalidColorToken {
                token: "panel",
                value: "rgba(1, 2, 300, 0.5)".to_string(),
            })
        );

        let mut bad_alpha = manifest.clone();
        bad_alpha.tokens.border = "rgba(1, 2, 3, 2)".to_string();
        assert_eq!(
            validate_theme_manifest(&bad_alpha),
            Err(ThemeManifestError::InvalidColorToken {
                token: "border",
                value: "rgba(1, 2, 3, 2)".to_string(),
            })
        );
    }
}

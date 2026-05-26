use super::primitives::{
    validate_color_token, validate_dotted_identifier, validate_non_empty, validate_simple_semver,
};
use crate::appearance_store::types::{ThemeManifest, ThemeTokens};

pub(super) fn validate_theme_manifest(theme: &ThemeManifest) -> Result<(), String> {
    validate_non_empty("theme id", &theme.id)?;
    validate_non_empty("theme name", &theme.name)?;
    validate_non_empty("theme version", &theme.version)?;
    validate_dotted_identifier("theme id", &theme.id, false)?;
    validate_simple_semver("theme version", &theme.version)?;
    validate_theme_tokens(&theme.tokens)
}

fn validate_theme_tokens(tokens: &ThemeTokens) -> Result<(), String> {
    validate_color_token("surface", &tokens.surface)?;
    validate_color_token("panel", &tokens.panel)?;
    validate_color_token("panelStrong", &tokens.panel_strong)?;
    validate_color_token("text", &tokens.text)?;
    validate_color_token("muted", &tokens.muted)?;
    validate_color_token("faint", &tokens.faint)?;
    validate_color_token("accent", &tokens.accent)?;
    validate_color_token("danger", &tokens.danger)?;
    validate_color_token("line", &tokens.line)?;
    validate_color_token("control", &tokens.control)?;
    validate_color_token("scrollbarThumb", &tokens.scrollbar_thumb)?;
    validate_color_token("scrollbarThumbHover", &tokens.scrollbar_thumb_hover)
}

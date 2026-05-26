use super::{DEFAULT_THEME_ID, types::*};
pub(super) fn built_in_theme_catalog() -> Vec<ThemeCatalogItem> {
    built_in_theme_manifests()
        .into_iter()
        .map(|theme| ThemeCatalogItem {
            id: theme.id,
            name: theme.name,
            version: theme.version,
            source: "builtIn".to_string(),
            plugin_id: None,
            enabled: true,
            tokens: theme.tokens,
        })
        .collect()
}

pub(super) fn built_in_theme_manifests() -> Vec<ThemeManifest> {
    vec![ThemeManifest {
        id: DEFAULT_THEME_ID.to_string(),
        name: "Studio Dark".to_string(),
        version: "1.0.0".to_string(),
        tokens: ThemeTokens {
            surface: "#050607".to_string(),
            panel: "rgba(8, 10, 12, 0.72)".to_string(),
            panel_strong: "rgba(8, 10, 12, 0.88)".to_string(),
            text: "#ece7dd".to_string(),
            muted: "#b9b0a3".to_string(),
            faint: "#8f867a".to_string(),
            accent: "#caa05d".to_string(),
            danger: "#d78372".to_string(),
            line: "rgba(236, 231, 221, 0.12)".to_string(),
            control: "rgba(18, 21, 25, 0.72)".to_string(),
            scrollbar_thumb: "rgba(236, 231, 221, 0.22)".to_string(),
            scrollbar_thumb_hover: "rgba(202, 160, 93, 0.46)".to_string(),
        },
    }]
}

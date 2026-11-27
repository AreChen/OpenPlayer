use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use gpui::{AssetSource, Rgba, SharedString, Svg, prelude::*, px, rgba, svg};
use serde::Deserialize;

const LOGO: &str = "openplayer-logo-transparent.svg";
const LOGO_SVG: &[u8] = include_bytes!("../../../src/assets/openplayer-logo-transparent.svg");

#[derive(Deserialize)]
struct IconPath {
    path: String,
    #[serde(default)]
    filled: bool,
}

static ICON_PATHS: LazyLock<HashMap<String, IconPath>> = LazyLock::new(|| {
    let shared: HashMap<String, String> =
        serde_json::from_str(include_str!("../../../src/app/icon-paths.json"))
            .expect("valid shared OpenPlayer icon paths");
    let native: HashMap<String, IconPath> =
        serde_json::from_str(include_str!("theme/native-icon-paths.json"))
            .expect("valid native OpenPlayer icon paths");
    shared
        .into_iter()
        .map(|(name, path)| {
            (
                name,
                IconPath {
                    path,
                    filled: false,
                },
            )
        })
        .chain(native)
        .collect()
});

pub(super) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path == LOGO {
            return Ok(Some(Cow::Borrowed(LOGO_SVG)));
        }
        let name = match path {
            "tracks" => "mediaTracks",
            "volume" => "volumeHigh",
            name => name,
        };
        Ok(ICON_PATHS.get(name).map(|icon| {
            let paint = if icon.filled {
                r#"fill="white""#
            } else {
                r#"fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round""#
            };
            Cow::Owned(format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" {paint}><path d="{}"/></svg>"#,
                icon.path,
            ).into_bytes())
        }))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICON_PATHS
            .keys()
            .map(|name| name.clone().into())
            .chain([LOGO.into()])
            .collect())
    }
}

pub(super) fn icon(name: impl Into<SharedString>, color: Rgba, size: f32) -> Svg {
    svg()
        .path(name)
        .size(px(size))
        .flex_none()
        .text_color(color)
}

pub(super) fn logo(accent: Rgba, size: f32) -> Svg {
    icon(LOGO, accent, size)
}

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub surface: Rgba,
    pub strong: Rgba,
    pub titlebar: Rgba,
    pub transport: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub faint: Rgba,
    pub accent: Rgba,
    pub accent_soft: Rgba,
    pub accent_muted: Rgba,
    pub accent_border: Rgba,
    pub danger: Rgba,
    pub line: Rgba,
    pub control: Rgba,
}

impl Palette {
    pub fn new(
        tokens: Option<&HashMap<String, String>>,
        titlebar_opacity: f64,
        transport_opacity: f64,
    ) -> Self {
        let color = |name, fallback| {
            tokens
                .and_then(|tokens| tokens.get(name))
                .and_then(|value| parse(value))
                .unwrap_or_else(|| rgba(fallback))
        };
        let accent = color("--accent", 0xcaa05dff);
        let panel = color("--panel", 0x080a0cb8);
        let strong = color("--panel-strong", 0x080a0ce0);
        Self {
            surface: color("--surface", 0x050607ff),
            strong,
            titlebar: Rgba {
                a: titlebar_opacity.clamp(0., 1.) as f32,
                ..panel
            },
            transport: Rgba {
                a: transport_opacity.clamp(0., 1.) as f32,
                ..strong
            },
            text: color("--text", 0xece7ddff),
            muted: color("--muted", 0xb9b0a3ff),
            faint: color("--faint", 0x8f867aff),
            accent,
            accent_soft: Rgba { a: 0.16, ..accent },
            accent_muted: Rgba { a: 0.22, ..accent },
            accent_border: Rgba { a: 0.42, ..accent },
            danger: color("--danger", 0xd78372ff),
            line: color("--line", 0xece7dd1f),
            control: color("--control", 0x121519b8),
        }
    }
}

pub(super) fn parse(value: &str) -> Option<Rgba> {
    if let Some(hex) = value.strip_prefix('#') {
        let number = u32::from_str_radix(hex, 16).ok()?;
        return match hex.len() {
            3 => Some(Rgba {
                r: ((number >> 8) & 15) as f32 / 15.,
                g: ((number >> 4) & 15) as f32 / 15.,
                b: (number & 15) as f32 / 15.,
                a: 1.,
            }),
            6 => Some(rgba((number << 8) | 255)),
            _ => None,
        };
    }
    let (parts, has_alpha) = match value.strip_prefix("rgba(") {
        Some(parts) => (parts, true),
        None => (value.strip_prefix("rgb(")?, false),
    };
    let mut parts = parts
        .strip_suffix(')')?
        .split(',')
        .map(|part| part.trim().parse::<f32>());
    Some(Rgba {
        r: parts.next()?.ok()? / 255.,
        g: parts.next()?.ok()? / 255.,
        b: parts.next()?.ok()? / 255.,
        a: if has_alpha { parts.next()?.ok()? } else { 1. },
    })
}

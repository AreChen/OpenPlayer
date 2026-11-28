use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::API_VERSION;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedText {
    pub en: String,
    pub zh_cn: String,
}

impl LocalizedText {
    pub fn new(en: impl Into<String>, zh_cn: impl Into<String>) -> Self {
        Self {
            en: en.into(),
            zh_cn: zh_cn.into(),
        }
    }

    pub fn get(&self, is_chinese: bool) -> &str {
        if is_chinese { &self.zh_cn } else { &self.en }
    }

    fn valid(&self) -> bool {
        [&self.en, &self.zh_cn]
            .into_iter()
            .all(|s| !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    OpenStream,
    Capture,
    SubtitleStyle,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum HostAction {
    OpenStreamDialog,
    CaptureScreenshot,
    SetSubtitleScale { scale: f64 },
}

impl HostAction {
    pub fn permission(self) -> Permission {
        match self {
            Self::OpenStreamDialog => Permission::OpenStream,
            Self::CaptureScreenshot => Permission::Capture,
            Self::SetSubtitleScale { .. } => Permission::SubtitleStyle,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub id: String,
    pub label: LocalizedText,
    pub command: HostAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Runtime {
    Wasm,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginManifest {
    pub api_version: u32,
    pub id: String,
    pub version: String,
    pub name: LocalizedText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<Runtime>,
    pub permissions: Vec<Permission>,
    pub actions: Vec<Action>,
}

fn identifier(text: &str) -> bool {
    text.len() <= 96
        && text.split('.').all(|segment| {
            segment
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_lowercase)
                && segment
                    .bytes()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'-')
        })
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.api_version != API_VERSION {
            return Err(format!("unsupported plugin API {}", self.api_version));
        }
        if !identifier(&self.id) || !self.id.contains('.') {
            return Err("expected a dotted lowercase plugin ID".into());
        }
        semver::Version::parse(&self.version).map_err(|err| err.to_string())?;
        if !self.name.valid() || !(1..=32).contains(&self.actions.len()) {
            return Err("expected a localized plugin name and 1–32 actions".into());
        }
        let permissions: HashSet<_> = self.permissions.iter().collect();
        if permissions.len() != self.permissions.len() {
            return Err("duplicate permission".into());
        }
        let mut ids = HashSet::new();
        for action in &self.actions {
            if !identifier(&action.id) || !ids.insert(&action.id) || !action.label.valid() {
                return Err(format!("invalid or duplicate action {}", action.id));
            }
            if !self.permissions.contains(&action.command.permission()) {
                return Err(format!(
                    "action {} lacks its required permission",
                    action.id
                ));
            }
            if let HostAction::SetSubtitleScale { scale } = action.command
                && !(0.25..=4.0).contains(&scale)
            {
                return Err("subtitle scale must be between 0.25 and 4".into());
            }
        }
        Ok(())
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|err| err.to_string())?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|err| err.to_string())
    }
}

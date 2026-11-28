use std::sync::LazyLock;

use serde_json::Value;
#[cfg(windows)]
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

static ENGLISH: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("locale/en-US.json")).expect("valid native English labels")
});
static CHINESE: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("locale/zh-CN.json")).expect("valid native Chinese labels")
});

pub(super) fn resolve_language(language: &str) -> &'static str {
    match language {
        "zh-CN" => "zh-CN",
        "en-US" => "en-US",
        #[cfg(unix)]
        _ => {
            let name = ["LC_ALL", "LC_MESSAGES", "LANG"]
                .iter()
                .filter_map(std::env::var_os)
                .find(|value| !value.is_empty());
            if name.is_some_and(|value| {
                value
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .starts_with("zh")
            }) {
                "zh-CN"
            } else {
                "en-US"
            }
        }
        #[cfg(windows)]
        _ => {
            const PRIMARY_LANGUAGE_MASK: u16 = 0x03ff;
            const LANG_CHINESE: u16 = 0x04;
            let ui_language = unsafe { GetUserDefaultUILanguage() };
            if ui_language & PRIMARY_LANGUAGE_MASK == LANG_CHINESE {
                "zh-CN"
            } else {
                "en-US"
            }
        }
    }
}

pub(super) fn labels(language: &str) -> &'static Value {
    match resolve_language(language) {
        "zh-CN" => &CHINESE,
        _ => &ENGLISH,
    }
}

pub(super) fn label(language: &str, key: &str) -> &'static str {
    key.split('.')
        .try_fold(labels(language), |labels, part| labels.get(part))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

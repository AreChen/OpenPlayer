use std::sync::LazyLock;

use serde_json::Value;
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

// Regenerate with apps/desktop/scripts/sync-native-locales.mjs.
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

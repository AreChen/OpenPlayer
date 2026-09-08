use super::ModuleLaunch;
use serde::Deserialize;
use std::{collections::HashMap, sync::LazyLock};

#[derive(Deserialize)]
struct Text {
    title: String,
    warning: String,
    executable: String,
    confirm: String,
}

static TEXT: LazyLock<HashMap<String, Text>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../src/i18n/nativePermission.json"))
        .expect("bundled native permission translations must be valid")
});

pub(super) fn message(launch: &ModuleLaunch) -> (String, String) {
    let chinese =
        launch.language_mode == "zh-CN" || (launch.language_mode == "system" && system_chinese());
    let text = &TEXT[if chinese { "zh-CN" } else { "en-US" }];
    (
        text.title.clone(),
        format!(
            "{}\n\n{} {} / {}\n\n{}: {}\n\n{}",
            text.warning,
            launch.plugin_name,
            launch.plugin_version,
            launch.module.id,
            text.executable,
            launch.executable.display(),
            text.confirm
        ),
    )
}

fn system_chinese() -> bool {
    #[cfg(windows)]
    {
        unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() & 0x3ff == 4 }
    }
    #[cfg(not(windows))]
    {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()))
            .is_some_and(|locale| locale.to_ascii_lowercase().starts_with("zh"))
    }
}

mod catalog;
mod mpv;
mod options;
mod values;

use super::primitives::{
    validate_dotted_identifier, validate_localized_text_map, validate_non_empty,
};
use crate::appearance_store::types::PluginSettingManifest;

pub(super) use mpv::is_allowed_plugin_mpv_property;
pub(super) use values::validate_plugin_setting_value;

pub(super) fn validate_plugin_setting(setting: &PluginSettingManifest) -> Result<(), String> {
    validate_non_empty("plugin setting id", &setting.id)?;
    validate_non_empty("plugin setting label", &setting.label)?;
    validate_dotted_identifier("plugin setting id", &setting.id, false)?;
    if let Some(description) = setting.description.as_deref() {
        validate_non_empty("plugin setting description", description)?;
    }
    validate_localized_text_map("plugin setting labelI18n", &setting.label_i18n, 128)?;
    validate_localized_text_map(
        "plugin setting descriptionI18n",
        &setting.description_i18n,
        512,
    )?;
    catalog::validate_setting_kind(&setting.kind)?;
    catalog::validate_setting_placement(&setting.placement)?;
    options::validate_setting_number_bounds(setting)?;
    options::validate_setting_options(setting)?;
    if let Some(property) = setting.mpv_property.as_deref() {
        mpv::validate_plugin_mpv_property(property)?;
        if setting.placement != "subtitleSettings" {
            return Err(format!(
                "mpv property setting {} must use subtitleSettings placement",
                setting.id
            ));
        }
    }
    validate_plugin_setting_value(setting, &setting.default_value)
}

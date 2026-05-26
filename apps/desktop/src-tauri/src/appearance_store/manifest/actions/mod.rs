mod args;
mod catalog;
mod stream;

use super::primitives::{
    validate_dotted_identifier, validate_localized_text_map, validate_non_empty,
};
use crate::appearance_store::types::PluginActionManifest;

use args::validate_plugin_action_args;
use catalog::{
    is_supported_action_placement, is_supported_plugin_action_command,
    is_supported_plugin_action_icon, plugin_action_required_permission,
};

pub(super) fn validate_plugin_action(
    action: &PluginActionManifest,
    permissions: &[String],
) -> Result<(), String> {
    validate_non_empty("plugin action id", &action.id)?;
    validate_non_empty("plugin action label", &action.label)?;
    validate_dotted_identifier("plugin action id", &action.id, false)?;
    if let Some(description) = action.description.as_deref() {
        validate_non_empty("plugin action description", description)?;
    }
    validate_localized_text_map("plugin action labelI18n", &action.label_i18n, 128)?;
    validate_localized_text_map(
        "plugin action descriptionI18n",
        &action.description_i18n,
        512,
    )?;
    if !is_supported_action_placement(&action.placement) {
        return Err(format!(
            "unsupported plugin action placement: {}",
            action.placement
        ));
    }
    if !is_supported_plugin_action_command(&action.command) {
        return Err(format!(
            "unsupported plugin action command: {}",
            action.command
        ));
    }
    if let Some(icon) = action.icon.as_deref()
        && !is_supported_plugin_action_icon(icon)
    {
        return Err(format!("unsupported plugin action icon: {icon}"));
    }
    if let Some(permission) = plugin_action_required_permission(&action.command)
        && !permissions.iter().any(|item| item == permission)
    {
        return Err(format!(
            "plugin action {} requires permission {}",
            action.id, permission
        ));
    }
    validate_plugin_action_args(action)?;
    Ok(())
}

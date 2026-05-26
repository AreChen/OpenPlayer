use std::fs;

use crate::appearance_store::{
    MAX_PLUGIN_VIEW_HTML_BYTES, manifest::validate_dotted_identifier,
    package::resolve_plugin_package_file_path, store::AppearanceStore, types::PluginViewHtml,
};

impl AppearanceStore {
    pub(in crate::appearance_store) fn plugin_view_html(
        &self,
        plugin_id: &str,
        view_id: &str,
    ) -> Result<PluginViewHtml, String> {
        let plugin_id = plugin_id.trim();
        let view_id = view_id.trim();
        validate_dotted_identifier("plugin id", plugin_id, true)?;
        validate_dotted_identifier("plugin view id", view_id, false)?;

        let manifest = self.plugin_manifest(plugin_id)?;
        let view = manifest
            .contributes
            .views
            .iter()
            .find(|view| view.id == view_id)
            .ok_or_else(|| format!("unknown plugin view: {plugin_id}.{view_id}"))?;
        let install = self
            .plugin_install_record(plugin_id)?
            .ok_or_else(|| format!("plugin {plugin_id} is not installed"))?;
        let html_path = resolve_plugin_package_file_path(&install.install_path, &view.entry)?;
        let metadata = fs::metadata(&html_path)
            .map_err(|error| format!("failed to inspect plugin view HTML: {error}"))?;
        if metadata.len() > MAX_PLUGIN_VIEW_HTML_BYTES {
            return Err(format!("plugin view HTML is too large: {}", view.entry));
        }
        let html = fs::read_to_string(&html_path)
            .map_err(|error| format!("failed to read plugin view HTML: {error}"))?;

        Ok(PluginViewHtml {
            plugin_id: manifest.id,
            view_id: view.id.clone(),
            title: view.title.clone(),
            html,
        })
    }
}

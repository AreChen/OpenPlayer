use std::path::PathBuf;

use crate::appearance_store::{records::current_time_ms, store::AppearanceStore};

impl AppearanceStore {
    pub(in crate::appearance_store) fn plugin_install_directory(&self, plugin_id: &str) -> PathBuf {
        self.plugin_root.join(plugin_id)
    }

    pub(in crate::appearance_store) fn plugin_staging_directory(&self, plugin_id: &str) -> PathBuf {
        self.plugin_root
            .join(format!(".{plugin_id}.installing-{}", current_time_ms()))
    }
}

use super::manifest::is_supported_plugin_permission;
use super::store::AppearanceStore;
use super::types::{PlayerPreferences, ThemeCatalogItem};
use super::*;
use std::path::PathBuf;

mod fixtures;
mod packages;
mod plugin_actions;
mod plugin_settings;
mod preferences;
mod runtime;
mod theme;

use fixtures::*;

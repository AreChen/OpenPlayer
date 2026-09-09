mod attachment;
#[cfg(all(windows, feature = "window-smoke"))]
pub(crate) mod attachment_smoke;
mod commands;
mod consent;
mod process_tree;
mod session;
#[cfg(test)]
pub(crate) mod tests;
mod types;
mod validation;
mod video_plan;

pub(crate) use commands::*;
use session::Session;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};
pub(crate) use types::{ModuleLaunch, NativeModule, NativeModuleInfo};
pub(crate) use validation::{current_target, validate_modules, verify_executable};

#[derive(Default)]
struct Registry {
    generation: u64,
    sessions: HashMap<(String, String), Arc<Session>>,
}

impl Registry {
    fn stop_matching(&mut self, matches: impl Fn(&(String, String)) -> bool) -> Result<(), String> {
        // Preserve records if any process has not exited, so the next operation
        // can retry cleanup instead of replacing files that are still in use.
        self.generation += 1;
        let keys: Vec<_> = self
            .sessions
            .keys()
            .filter(|key| matches(key))
            .cloned()
            .collect();
        for key in &keys {
            self.sessions[key].tree.stop();
        }
        for key in &keys {
            self.sessions[key].stop_and_wait()?;
        }
        for key in keys {
            self.sessions.remove(&key);
        }
        Ok(())
    }

    fn prune_exited(&mut self) -> Result<(), String> {
        let keys: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, session)| !session.running())
            .map(|(key, _)| key.clone())
            .collect();
        for key in &keys {
            self.sessions[key].stop_and_wait()?;
        }
        for key in keys {
            self.sessions.remove(&key);
        }
        Ok(())
    }
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| Mutex::new(Registry::default()));

pub(crate) fn invalidate_plugin(plugin_id: &str) -> Result<(), String> {
    REGISTRY
        .lock()
        .map_err(|_| "native registry unavailable")?
        .stop_matching(|(id, _)| id == plugin_id)
}

pub(crate) fn shutdown() {
    if let Ok(mut registry) = REGISTRY.lock() {
        registry.generation += 1;
        for session in registry.sessions.values() {
            session.tree.stop();
        }
        registry.sessions.clear();
    }
}

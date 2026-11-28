//! SDK 2 package discovery and permission-checked action dispatch.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read},
    path::Path,
    sync::OnceLock,
};

pub use openplayer_plugin_sdk::{HostAction, PlaybackContext};
use openplayer_plugin_sdk::{LocalizedText, PluginManifest};
use sha2::{Digest, Sha256};
use wasmi::{Config, EnforcedLimits, Engine, Linker, Module, Store, StoreLimitsBuilder};

const PACKAGE_SUFFIX: &str = ".opplugin.json";
const MAX_MANIFEST: usize = 64 * 1024;
const MAX_WASM: usize = 256 * 1024;
const MAX_USER_PACKAGES: usize = 64;

type Builtin = (&'static [u8], Option<&'static [u8]>);
const BUILTINS: &[Builtin] = &[
    (
        include_bytes!("../assets/plugins/network-stream.opplugin.json"),
        None,
    ),
    (
        include_bytes!("../assets/plugins/capture-tools.opplugin.json"),
        None,
    ),
    (
        include_bytes!("../assets/plugins/subtitle-typography.opplugin.json"),
        Some(include_bytes!("../assets/plugins/subtitle-typography.wasm")),
    ),
];

struct Guest {
    bytes: Vec<u8>,
    module: OnceLock<Result<Module, String>>,
}

impl Guest {
    fn invoke(&self, action: usize, ctx: PlaybackContext) -> anyhow::Result<usize> {
        let module = self
            .module
            .get_or_init(|| {
                let mut config = Config::default();
                config
                    .consume_fuel(true)
                    .enforced_limits(EnforcedLimits::strict());
                let engine = Engine::new(&config);
                let module = Module::new(&engine, &self.bytes).map_err(|e| e.to_string())?;
                if module.imports().next().is_some() {
                    return Err("SDK 2 guests cannot import external functions or memory".into());
                }
                Ok(module)
            })
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(4 * 1024 * 1024)
            .table_elements(256)
            .memories(1)
            .tables(1)
            .instances(1)
            .build();
        let mut store = Store::new(module.engine(), limits);
        store.limiter(|limits| limits);
        store.set_fuel(100_000)?;
        let instance = Linker::new(module.engine()).instantiate_and_start(&mut store, module)?;
        let run = instance
            .get_typed_func::<(u32, u32, u64, u64, f64), i32>(&store, "openplayer_invoke_v2")?;
        let index = run.call(
            &mut store,
            (
                action as u32,
                ctx.flags(),
                ctx.position_ms,
                ctx.duration_ms,
                ctx.subtitle_scale,
            ),
        )?;
        usize::try_from(index)
            .map_err(|_| anyhow::anyhow!("plugin has no command for this playback state"))
    }
}

struct Package {
    manifest: PluginManifest,
    guest: Option<Guest>,
    bundled: bool,
    approval_key: String,
}

pub struct PluginAction<'a> {
    pub plugin_id: &'a str,
    pub id: &'a str,
    pub label: &'a LocalizedText,
}

#[derive(Default)]
pub struct PluginRegistry {
    packages: BTreeMap<String, Package>,
    diagnostics: Vec<String>,
}

impl PluginRegistry {
    /// Load packaged plugins and take a snapshot of the user directory. Editors
    /// may replace its contents while OpenPlayer is running; a rescan picks up
    /// changes without modifying a previously approved snapshot.
    pub fn load(directory: &Path) -> Self {
        let mut registry = Self::default();
        for &(manifest, wasm) in BUILTINS {
            if let Err(error) = PluginManifest::from_json(manifest)
                .and_then(|manifest| registry.add(manifest, wasm.map(<[u8]>::to_vec), true))
            {
                registry
                    .diagnostics
                    .push(format!("Bundled plugin: {error}"));
            }
        }
        if let Err(error) = registry.scan(directory) {
            registry.diagnostics.push(error.to_string());
        }
        registry
    }

    fn scan(&mut self, directory: &Path) -> io::Result<()> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let mut files = BTreeSet::new();
        let mut overflow = false;
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    self.diagnostics.push(error.to_string());
                    continue;
                }
            };
            if package_stem(&path).is_some() {
                files.insert(path);
                if files.len() > MAX_USER_PACKAGES {
                    files.pop_last();
                    overflow = true;
                }
            }
        }
        if overflow {
            self.diagnostics.push(format!(
                "Only the first {MAX_USER_PACKAGES} plugin packages by filename can be loaded"
            ));
        }
        for path in files {
            if let Err(error) = self.load_file(&path) {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                self.diagnostics.push(format!("{filename}: {error}"));
            }
        }
        Ok(())
    }

    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let bytes = read_file(path, MAX_MANIFEST).map_err(|e| e.to_string())?;
        let manifest = PluginManifest::from_json(&bytes)?;
        let wasm = if manifest.runtime.is_some() {
            let stem = package_stem(path).ok_or("invalid package filename")?;
            let wasm_path = path.with_file_name(format!("{stem}.wasm"));
            Some(
                read_file(&wasm_path, MAX_WASM)
                    .map_err(|e| format!("cannot read guest module: {e}"))?,
            )
        } else {
            None
        };
        self.add(manifest, wasm, false)
    }

    fn add(
        &mut self,
        manifest: PluginManifest,
        wasm: Option<Vec<u8>>,
        bundled: bool,
    ) -> Result<(), String> {
        if self.packages.contains_key(&manifest.id) {
            return Err(format!("duplicate plugin ID {}", manifest.id));
        }
        if manifest.runtime.is_some() != wasm.is_some() {
            return Err("manifest and guest module disagree".into());
        }
        let approval_key = if bundled {
            manifest.id.clone()
        } else {
            let mut digest = Sha256::new();
            digest.update(manifest.to_json()?.as_bytes());
            digest.update([0]);
            digest.update(wasm.as_deref().unwrap_or_default());
            format!("{}@{}", manifest.id, hex::encode(digest.finalize()))
        };
        let guest = wasm.map(|bytes| Guest {
            bytes,
            module: OnceLock::new(),
        });
        self.packages.insert(
            manifest.id.clone(),
            Package {
                manifest,
                guest,
                bundled,
                approval_key,
            },
        );
        Ok(())
    }

    pub fn plugins(&self) -> impl Iterator<Item = &PluginManifest> {
        self.packages.values().map(|p| &p.manifest)
    }

    pub fn actions(&self) -> impl Iterator<Item = PluginAction<'_>> {
        self.plugins().flat_map(|p| {
            p.actions.iter().map(|a| PluginAction {
                plugin_id: &p.id,
                id: &a.id,
                label: &a.label,
            })
        })
    }

    pub fn is_bundled(&self, id: &str) -> bool {
        self.packages.get(id).is_some_and(|p| p.bundled)
    }

    pub fn approval_key(&self, id: &str) -> Option<&str> {
        self.packages.get(id).map(|p| p.approval_key.as_str())
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn invoke(
        &self,
        plugin_id: &str,
        action_id: &str,
        context: PlaybackContext,
    ) -> Result<HostAction, String> {
        let package = self.packages.get(plugin_id).ok_or("unknown plugin")?;
        let manifest = &package.manifest;
        let index = manifest
            .actions
            .iter()
            .position(|a| a.id == action_id)
            .ok_or("unknown plugin action")?;
        let selected = match &package.guest {
            Some(guest) => guest.invoke(index, context).map_err(|e| e.to_string())?,
            None => index,
        };
        let action = manifest
            .actions
            .get(selected)
            .ok_or("guest selected an undeclared command")?;
        if !manifest.permissions.contains(&action.command.permission()) {
            return Err("plugin lacks the required permission".into());
        }
        Ok(action.command)
    }
}

fn package_stem(path: &Path) -> Option<&str> {
    path.file_name()?
        .to_str()?
        .strip_suffix(PACKAGE_SUFFIX)
        .filter(|s| !s.is_empty())
}

fn read_file(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Err(io::Error::other(
            "expected a regular file within the package size limit",
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::other("package size limit exceeded"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const SUBTITLES: &str = "dev.openplayer.subtitle.typography";
    const CAPTURE: &str = "dev.openplayer.capture.tools";

    #[test]
    fn bundled_manifests_dispatch_native_and_rust_wasm_actions() {
        fn thread_safe<T: Send + Sync>() {}
        thread_safe::<PluginRegistry>();
        let tmp = tempdir().unwrap();
        let registry = PluginRegistry::load(&tmp.path().join("missing"));
        assert!(
            registry.diagnostics().is_empty(),
            "{:?}",
            registry.diagnostics()
        );
        assert!(registry.is_bundled(CAPTURE));
        assert_eq!(registry.approval_key(CAPTURE), Some(CAPTURE));
        let context = PlaybackContext::default();
        assert_eq!(
            registry.invoke(CAPTURE, "save-screenshot", context),
            Ok(HostAction::CaptureScreenshot)
        );
        assert_eq!(
            registry.invoke("dev.openplayer.network.stream", "open-stream", context),
            Ok(HostAction::OpenStreamDialog)
        );
        for (scale, expected) in [(0.8, 1.0), (1.0, 1.25)] {
            let context = PlaybackContext {
                subtitle_scale: scale,
                ..context
            };
            assert_eq!(
                registry.invoke(SUBTITLES, "larger", context),
                Ok(HostAction::SetSubtitleScale { scale: expected })
            );
        }
        assert!(registry.invoke(CAPTURE, "missing", context).is_err());
    }

    #[test]
    fn a_rescan_preserves_valid_packages_and_rejects_shadowing() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("broken.opplugin.json"), "{}").unwrap();
        fs::write(tmp.path().join("duplicate.opplugin.json"), BUILTINS[1].0).unwrap();
        let mut manifest = PluginManifest::from_json(BUILTINS[1].0).unwrap();
        manifest.id = "example.capture".into();
        let user_file = tmp.path().join("user.opplugin.json");
        fs::write(&user_file, manifest.to_json().unwrap()).unwrap();
        let registry = PluginRegistry::load(tmp.path());
        assert_eq!(registry.diagnostics().len(), 2);
        assert!(!registry.is_bundled(&manifest.id));
        let key = registry.approval_key(&manifest.id).unwrap();
        assert!(key.starts_with("example.capture@"));
        assert_eq!(
            PluginRegistry::load(tmp.path()).approval_key(&manifest.id),
            Some(key)
        );
        manifest.version = "1.0.1".into();
        fs::write(&user_file, manifest.to_json().unwrap()).unwrap();
        assert_ne!(
            PluginRegistry::load(tmp.path()).approval_key(&manifest.id),
            Some(key)
        );
    }

    #[test]
    fn external_module_bytes_are_part_of_the_approved_snapshot() {
        let tmp = tempdir().unwrap();
        let (json, wasm) = BUILTINS[2];
        let mut manifest = PluginManifest::from_json(json).unwrap();
        manifest.id = "example.subtitles".into();
        fs::write(
            tmp.path().join("custom.opplugin.json"),
            manifest.to_json().unwrap(),
        )
        .unwrap();
        let wasm_path = tmp.path().join("custom.wasm");
        let missing = PluginRegistry::load(tmp.path());
        assert_eq!(missing.diagnostics().len(), 1);
        fs::write(&wasm_path, wasm.unwrap()).unwrap();
        let first = PluginRegistry::load(tmp.path());
        let mut replacement = wasm.unwrap().to_vec();
        // Append an empty, named custom section; this is a different, valid guest.
        replacement.extend_from_slice(&[0, 2, 1, b'v']);
        fs::write(&wasm_path, replacement).unwrap();
        let second = PluginRegistry::load(tmp.path());
        assert_ne!(
            first.approval_key(&manifest.id),
            second.approval_key(&manifest.id)
        );
        for registry in [&first, &second] {
            assert_eq!(
                registry.invoke(&manifest.id, "larger", PlaybackContext::default()),
                Ok(HostAction::SetSubtitleScale { scale: 1.25 })
            );
        }
    }
}

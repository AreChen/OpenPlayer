use super::inventory::{Inventory, plain};
use std::{
    fs::{self, File, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) struct RuntimeCopy {
    pub path: PathBuf,
    pub digest: String,
    _lease: File,
    retained: bool,
}

impl RuntimeCopy {
    pub fn prepare(source: &Path, root: &Path, inventory: &Inventory) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err("runtime cache must be absolute".into());
        }
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        if !plain(root)?.is_dir() {
            return Err("runtime cache must be a directory".into());
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let source = source.canonicalize().map_err(|e| e.to_string())?;
        if root.starts_with(&source) || source.starts_with(&root) {
            return Err("runtime cache overlaps source".into());
        }
        cleanup(&root)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let path = root.join(format!("runtime-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(4)
            .open(path.join(".lease"))
            .map_err(|e| e.to_string())?;
        let mut copy = Self {
            path,
            digest: inventory.digest.clone(),
            _lease: lease,
            retained: false,
        };
        // Content lives below the lease so payload verification is independent
        // of cache coordination files and never changes the installed package.
        let content = copy.path.join("content");
        fs::create_dir(&content).map_err(|e| e.to_string())?;
        inventory.copy_to(&source, &content)?;
        copy.path = content;
        Ok(copy)
    }

    pub fn retain_until_exit(&mut self) {
        self.retained = true;
    }
}

impl Drop for RuntimeCopy {
    fn drop(&mut self) {
        if !self.retained {
            let path = if self.path.file_name().is_some_and(|n| n == "content") {
                self.path.parent().unwrap_or(&self.path)
            } else {
                &self.path
            };
            let _ = fs::remove_dir_all(path);
        }
    }
}

pub(super) fn cleanup(root: &Path) -> Result<(), String> {
    for (index, entry) in fs::read_dir(root).map_err(|e| e.to_string())?.enumerate() {
        if index >= 64 {
            return Err("native runtime cache entry limit reached".into());
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let parts: Vec<_> = name.split('-').collect();
        if parts.len() != 3
            || parts[0] != "runtime"
            || parts[1..]
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            continue;
        }
        if !plain(&path)?.is_dir() {
            continue;
        }
        let lock = path.join(".lease");
        if !lock.exists() {
            continue;
        }
        if !plain(&lock)?.is_file() {
            return Err("invalid runtime lease".into());
        }
        match OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(4)
            .open(lock)
        {
            Ok(_lease) => {
                fs::remove_dir_all(path)
                    .map_err(|e| format!("runtime cache cleanup failed: {e}"))?;
            }
            Err(error) if matches!(error.raw_os_error(), Some(32 | 33)) => (),
            Err(error) => return Err(format!("runtime cache lease failed: {error}")),
        }
    }
    Ok(())
}

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_FILES: usize = 1024;
const INVENTORY: &str = "bundle-inventory.json";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    schema: u32,
    #[serde(default)]
    metadata: BTreeMap<String, serde_json::Value>,
    files: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    size: u64,
    sha256: String,
}

pub(super) struct Inventory {
    pub digest: String,
    pub files: BTreeMap<String, Entry>,
    raw: Vec<u8>,
}

pub(super) fn plain(path: &Path) -> Result<fs::Metadata, String> {
    use std::os::windows::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err("runtime reparse points are not allowed".into());
    }
    Ok(metadata)
}

fn relative(name: &str) -> Result<PathBuf, String> {
    if name.len() > 240
        || name.contains(['\\', ':'])
        || name.split('/').any(|part| {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part
                    .chars()
                    .any(|c| c.is_control() || "<>\"|?*".contains(c))
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.as_bytes()[3].is_ascii_digit())
        })
    {
        return Err("invalid runtime inventory path".into());
    }
    Ok(PathBuf::from(name))
}

impl Inventory {
    pub fn read(source: &Path) -> Result<Self, String> {
        if !source.is_absolute() || !plain(source)?.is_dir() {
            return Err("runtime source must be an absolute directory".into());
        }
        let path = source.join(INVENTORY);
        if !plain(&path)?.is_file() {
            return Err("runtime inventory must be a file".into());
        }
        let mut raw = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(256 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        if raw.len() > 256 * 1024 {
            return Err("runtime inventory too large".into());
        }
        let document: Document = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        if document.schema != 2
            || document.files.is_empty()
            || document.files.len() >= MAX_FILES
            || document.metadata.len() > 32
        {
            return Err("unsupported runtime inventory".into());
        }
        let mut bytes = raw.len() as u64;
        let mut names = BTreeSet::new();
        for (name, entry) in &document.files {
            relative(name)?;
            if name.eq_ignore_ascii_case(INVENTORY)
                || !names.insert(name.to_lowercase())
                || entry.sha256.len() != 64
                || !entry.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err("invalid or colliding runtime entry".into());
            }
            bytes = bytes
                .checked_add(entry.size)
                .ok_or("runtime size overflow")?;
            if bytes > MAX_BYTES {
                return Err("runtime exceeds package size limit".into());
            }
        }
        Ok(Self {
            digest: format!("{:x}", Sha256::digest(&raw)),
            files: document.files,
            raw,
        })
    }

    pub fn copy_to(&self, source: &Path, destination: &Path) -> Result<(), String> {
        let mut actual = BTreeSet::new();
        collect(source, source, &mut actual, 0, &mut 0)?;
        let expected = self
            .files
            .keys()
            .cloned()
            .chain(Some(INVENTORY.into()))
            .collect();
        if actual != expected {
            return Err("runtime contains missing or undeclared files".into());
        }
        for (name, entry) in &self.files {
            let input_path = source.join(relative(name)?);
            if plain(&input_path)?.len() != entry.size {
                return Err(format!("runtime size mismatch: {name}"));
            }
            let output_path = destination.join(relative(name)?);
            fs::create_dir_all(output_path.parent().ok_or("invalid runtime path")?)
                .map_err(|e| e.to_string())?;
            let mut input = fs::File::open(input_path)
                .map_err(|e| e.to_string())?
                .take(entry.size + 1);
            let mut output = fs::File::create_new(output_path).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut copied = 0;
            let mut buffer = [0u8; 65536];
            loop {
                let length = input.read(&mut buffer).map_err(|e| e.to_string())?;
                if length == 0 {
                    break;
                }
                copied += length as u64;
                if copied > entry.size {
                    return Err("runtime file grew during copy".into());
                }
                hash.update(&buffer[..length]);
                output
                    .write_all(&buffer[..length])
                    .map_err(|e| e.to_string())?;
            }
            if copied != entry.size
                || !format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&entry.sha256)
            {
                return Err(format!("runtime integrity mismatch: {name}"));
            }
        }
        fs::write(destination.join(INVENTORY), &self.raw).map_err(|e| e.to_string())
    }
}

fn collect(
    root: &Path,
    path: &Path,
    files: &mut BTreeSet<String>,
    depth: usize,
    visits: &mut usize,
) -> Result<(), String> {
    if depth > 32 {
        return Err("runtime directory nesting too deep".into());
    }
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        *visits += 1;
        if *visits > MAX_FILES * 2 {
            return Err("too many runtime directory entries".into());
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        let metadata = plain(&path)?;
        if metadata.is_dir() {
            collect(root, &path, files, depth + 1, visits)?;
        } else if metadata.is_file() {
            let name = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("runtime file name is not UTF-8")?
                .replace('\\', "/");
            files.insert(name);
            if files.len() > MAX_FILES {
                return Err("too many runtime files".into());
            }
        } else {
            return Err("runtime contains a non-regular file".into());
        }
    }
    Ok(())
}

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::Manager;

const AUDIO_CLIP_ROOT: &str = "audio-clips";
const FRAME_CAPTURE_ROOT: &str = "frame-captures";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginArtifactKind {
    AudioClip,
    FrameCapture,
}

impl PluginArtifactKind {
    fn all() -> [Self; 2] {
        [Self::AudioClip, Self::FrameCapture]
    }

    fn from_option(value: Option<String>) -> Result<Option<Self>, String> {
        value.as_deref().map(Self::from_str).transpose()
    }

    fn from_str(value: &str) -> Result<Self, String> {
        match value.trim() {
            "audioClip" => Ok(Self::AudioClip),
            "frameCapture" => Ok(Self::FrameCapture),
            _ => Err("plugin artifact kind must be audioClip or frameCapture".to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::AudioClip => "audioClip",
            Self::FrameCapture => "frameCapture",
        }
    }

    fn root(self) -> &'static str {
        match self {
            Self::AudioClip => AUDIO_CLIP_ROOT,
            Self::FrameCapture => FRAME_CAPTURE_ROOT,
        }
    }

    fn mime_type_for_extension(self, extension: &str) -> Option<&'static str> {
        match (self, extension) {
            (Self::AudioClip, "wav") => Some("audio/wav"),
            (Self::FrameCapture, "png") => Some("image/png"),
            (Self::FrameCapture, "jpg" | "jpeg") => Some("image/jpeg"),
            (Self::FrameCapture, "webp") => Some("image/webp"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginArtifactInfo {
    kind: String,
    path: String,
    file_name: String,
    mime_type: String,
    size_bytes: u64,
    created_at_ms: Option<u64>,
    modified_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginArtifactClearResult {
    removed_count: usize,
    bytes_freed: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginArtifactRemoveResult {
    removed: bool,
    bytes_freed: u64,
}

#[tauri::command]
pub(crate) fn plugin_artifacts_list(
    app: tauri::AppHandle,
    plugin_id: String,
    kind: Option<String>,
) -> Result<Vec<PluginArtifactInfo>, String> {
    let app_data_dir = app_data_dir(&app)?;
    let kind = PluginArtifactKind::from_option(kind)?;
    list_plugin_artifacts(&app_data_dir, &plugin_id, kind)
}

#[tauri::command]
pub(crate) fn plugin_artifacts_info(
    app: tauri::AppHandle,
    plugin_id: String,
    path: String,
) -> Result<Option<PluginArtifactInfo>, String> {
    let app_data_dir = app_data_dir(&app)?;
    plugin_artifact_info(&app_data_dir, &plugin_id, &path)
}

#[tauri::command]
pub(crate) fn plugin_artifacts_remove(
    app: tauri::AppHandle,
    plugin_id: String,
    path: String,
) -> Result<PluginArtifactRemoveResult, String> {
    let app_data_dir = app_data_dir(&app)?;
    remove_plugin_artifact(&app_data_dir, &plugin_id, &path)
}

#[tauri::command]
pub(crate) fn plugin_artifacts_clear(
    app: tauri::AppHandle,
    plugin_id: String,
    kind: Option<String>,
) -> Result<PluginArtifactClearResult, String> {
    let app_data_dir = app_data_dir(&app)?;
    let kind = PluginArtifactKind::from_option(kind)?;
    clear_plugin_artifacts_for_plugin(&app_data_dir, &plugin_id, kind)
}

pub(crate) fn clear_plugin_artifacts_for_plugin(
    app_data_dir: &Path,
    plugin_id: &str,
    kind: Option<PluginArtifactKind>,
) -> Result<PluginArtifactClearResult, String> {
    let plugin_id = validate_plugin_artifact_plugin_id(plugin_id)?;
    let kinds: Vec<_> = kind
        .map(|kind| vec![kind])
        .unwrap_or_else(|| PluginArtifactKind::all().to_vec());
    let mut result = PluginArtifactClearResult {
        removed_count: 0,
        bytes_freed: 0,
    };
    for kind in kinds {
        let directory = artifact_directory(app_data_dir, plugin_id, kind);
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!("failed to read plugin artifact directory entry: {error}")
            })?;
            let path = entry.path();
            if let Some(info) = artifact_info_from_path(kind, &path)? {
                fs::remove_file(&path)
                    .map_err(|error| format!("failed to remove plugin artifact: {error}"))?;
                result.removed_count += 1;
                result.bytes_freed = result.bytes_freed.saturating_add(info.size_bytes);
            }
        }
        let _ = fs::remove_dir(&directory);
    }
    Ok(result)
}

fn list_plugin_artifacts(
    app_data_dir: &Path,
    plugin_id: &str,
    kind: Option<PluginArtifactKind>,
) -> Result<Vec<PluginArtifactInfo>, String> {
    let plugin_id = validate_plugin_artifact_plugin_id(plugin_id)?;
    let kinds: Vec<_> = kind
        .map(|kind| vec![kind])
        .unwrap_or_else(|| PluginArtifactKind::all().to_vec());
    let mut artifacts = Vec::new();
    for kind in kinds {
        let directory = artifact_directory(app_data_dir, plugin_id, kind);
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!("failed to read plugin artifact directory entry: {error}")
            })?;
            if let Some(artifact) = artifact_info_from_path(kind, &entry.path())? {
                artifacts.push(artifact);
            }
        }
    }
    artifacts.sort_by(|left, right| {
        right
            .modified_at_ms
            .cmp(&left.modified_at_ms)
            .then(left.path.cmp(&right.path))
    });
    Ok(artifacts)
}

fn plugin_artifact_info(
    app_data_dir: &Path,
    plugin_id: &str,
    path: &str,
) -> Result<Option<PluginArtifactInfo>, String> {
    let (kind, artifact_path) = checked_plugin_artifact_path(app_data_dir, plugin_id, path)?;
    artifact_info_from_path(kind, &artifact_path)
}

fn remove_plugin_artifact(
    app_data_dir: &Path,
    plugin_id: &str,
    path: &str,
) -> Result<PluginArtifactRemoveResult, String> {
    let (kind, artifact_path) = checked_plugin_artifact_path(app_data_dir, plugin_id, path)?;
    let Some(info) = artifact_info_from_path(kind, &artifact_path)? else {
        return Ok(PluginArtifactRemoveResult {
            removed: false,
            bytes_freed: 0,
        });
    };
    fs::remove_file(&artifact_path)
        .map_err(|error| format!("failed to remove plugin artifact: {error}"))?;
    Ok(PluginArtifactRemoveResult {
        removed: true,
        bytes_freed: info.size_bytes,
    })
}

fn checked_plugin_artifact_path(
    app_data_dir: &Path,
    plugin_id: &str,
    path: &str,
) -> Result<(PluginArtifactKind, PathBuf), String> {
    let plugin_id = validate_plugin_artifact_plugin_id(plugin_id)?;
    if path.trim().is_empty() || path.len() > 4096 {
        return Err("plugin artifact path is required".to_string());
    }
    let requested_path = PathBuf::from(path);
    if !requested_path.is_absolute() {
        return Err("plugin artifact path must be absolute".to_string());
    }
    if fs::symlink_metadata(&requested_path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("plugin artifact must be a managed file".to_string());
    }
    let canonical_path = match fs::canonicalize(&requested_path) {
        Ok(path) => path,
        Err(_) => return Ok((PluginArtifactKind::AudioClip, requested_path)),
    };
    for kind in PluginArtifactKind::all() {
        let directory = artifact_directory(app_data_dir, plugin_id, kind);
        if fs::canonicalize(directory)
            .ok()
            .is_some_and(|root| canonical_path.starts_with(root))
        {
            return Ok((kind, canonical_path));
        }
    }
    Err("plugin artifact must belong to the current plugin".to_string())
}

fn artifact_info_from_path(
    kind: PluginArtifactKind,
    path: &Path,
) -> Result<Option<PluginArtifactInfo>, String> {
    let Some(file_name) = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let Some(mime_type) = kind.mime_type_for_extension(&extension) else {
        return Ok(None);
    };
    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return Ok(None),
        Err(_) => return Ok(None),
    };
    Ok(Some(PluginArtifactInfo {
        kind: kind.as_str().to_string(),
        path: path.to_string_lossy().to_string(),
        file_name: file_name.to_string(),
        mime_type: mime_type.to_string(),
        size_bytes: metadata.len(),
        created_at_ms: metadata.created().ok().and_then(system_time_ms),
        modified_at_ms: metadata.modified().ok().and_then(system_time_ms),
    }))
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))
}

fn artifact_directory(app_data_dir: &Path, plugin_id: &str, kind: PluginArtifactKind) -> PathBuf {
    app_data_dir.join(kind.root()).join(plugin_id)
}

fn validate_plugin_artifact_plugin_id(plugin_id: &str) -> Result<&str, String> {
    let plugin_id = plugin_id.trim();
    if plugin_id.len() > 128
        || !plugin_id.contains('.')
        || plugin_id.split('.').any(|segment| {
            segment.is_empty()
                || !segment
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_lowercase())
                || !segment.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
        })
    {
        return Err("plugin artifact invalid plugin id".to_string());
    }
    Ok(plugin_id)
}

fn system_time_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_filters_current_plugin_artifacts() {
        let directory = test_directory("lists_and_filters_current_plugin_artifacts");
        let audio_dir = directory
            .join(AUDIO_CLIP_ROOT)
            .join("dev.openplayer.ai-transcript");
        let frame_dir = directory
            .join(FRAME_CAPTURE_ROOT)
            .join("dev.openplayer.ai-transcript");
        fs::create_dir_all(&audio_dir).expect("audio artifact dir should be created");
        fs::create_dir_all(&frame_dir).expect("frame artifact dir should be created");
        fs::write(audio_dir.join("clip.wav"), b"wav").expect("audio artifact should be written");
        fs::write(frame_dir.join("frame.webp"), b"webp").expect("frame artifact should be written");
        fs::write(frame_dir.join("ignore.txt"), b"text")
            .expect("unsupported file should be written");

        let all = list_plugin_artifacts(&directory, "dev.openplayer.ai-transcript", None)
            .expect("artifacts should list");
        let audio = list_plugin_artifacts(
            &directory,
            "dev.openplayer.ai-transcript",
            Some(PluginArtifactKind::AudioClip),
        )
        .expect("audio artifacts should list");
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|artifact| artifact.kind == "audioClip"));
        assert!(all.iter().any(|artifact| artifact.kind == "frameCapture"));
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].mime_type, "audio/wav");
    }

    #[test]
    fn rejects_cross_plugin_or_unmanaged_artifact_paths() {
        let directory = test_directory("rejects_cross_plugin_or_unmanaged_artifact_paths");
        let own_dir = directory
            .join(AUDIO_CLIP_ROOT)
            .join("dev.openplayer.ai-transcript");
        let other_dir = directory.join(AUDIO_CLIP_ROOT).join("dev.openplayer.other");
        fs::create_dir_all(&own_dir).expect("own artifact dir should be created");
        fs::create_dir_all(&other_dir).expect("other artifact dir should be created");
        let own_path = own_dir.join("clip.wav");
        let other_path = other_dir.join("clip.wav");
        let unmanaged_path = directory.join("clip.wav");
        fs::write(&own_path, b"wav").expect("own artifact should be written");
        fs::write(&other_path, b"wav").expect("other artifact should be written");
        fs::write(&unmanaged_path, b"wav").expect("unmanaged artifact should be written");

        assert!(
            plugin_artifact_info(
                &directory,
                "dev.openplayer.ai-transcript",
                &own_path.to_string_lossy()
            )
            .expect("own artifact should be readable")
            .is_some()
        );
        assert!(
            plugin_artifact_info(
                &directory,
                "dev.openplayer.ai-transcript",
                &other_path.to_string_lossy()
            )
            .expect_err("cross-plugin artifact should reject")
            .contains("current plugin")
        );
        let error = plugin_artifact_info(
            &directory,
            "dev.openplayer.ai-transcript",
            &unmanaged_path.to_string_lossy(),
        )
        .expect_err("unmanaged artifact should reject");
        let _ = fs::remove_dir_all(&directory);

        assert!(error.contains("current plugin"));
    }

    #[test]
    fn removes_and_clears_current_plugin_artifacts() {
        let directory = test_directory("removes_and_clears_current_plugin_artifacts");
        let audio_dir = directory
            .join(AUDIO_CLIP_ROOT)
            .join("dev.openplayer.ai-transcript");
        let frame_dir = directory
            .join(FRAME_CAPTURE_ROOT)
            .join("dev.openplayer.ai-transcript");
        fs::create_dir_all(&audio_dir).expect("audio artifact dir should be created");
        fs::create_dir_all(&frame_dir).expect("frame artifact dir should be created");
        let clip_path = audio_dir.join("clip.wav");
        fs::write(&clip_path, b"wav").expect("audio artifact should be written");
        fs::write(frame_dir.join("frame.png"), b"png").expect("frame artifact should be written");

        let remove_result = remove_plugin_artifact(
            &directory,
            "dev.openplayer.ai-transcript",
            &clip_path.to_string_lossy(),
        )
        .expect("artifact remove should succeed");
        assert!(remove_result.removed);
        assert_eq!(remove_result.bytes_freed, 3);
        assert!(!clip_path.exists());

        let clear_result =
            clear_plugin_artifacts_for_plugin(&directory, "dev.openplayer.ai-transcript", None)
                .expect("artifact clear should succeed");
        let remaining = list_plugin_artifacts(&directory, "dev.openplayer.ai-transcript", None)
            .expect("remaining artifacts should list");
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(clear_result.removed_count, 1);
        assert_eq!(clear_result.bytes_freed, 3);
        assert!(remaining.is_empty());
    }

    fn test_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-plugin-artifacts-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test time should be after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&directory);
        directory
    }
}

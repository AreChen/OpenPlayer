import { invoke } from "@tauri-apps/api/core";
import { clampPlaybackSpeed, hwdecModeFromSnapshot } from "./playback";
import type { MediaPlaybackSettings, MpvSnapshot, PlaybackSettings } from "./types";

export async function applyStoredPlaybackSettingsToMpv(snapshot: MpvSnapshot, settings: PlaybackSettings) {
  let activeSnapshot = snapshot;
  if (Math.abs(activeSnapshot.volume - settings.volume) > 0.5) {
    activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_volume", { volume: settings.volume });
  }
  if (Math.abs(clampPlaybackSpeed(activeSnapshot.speed) - settings.playbackSpeed) > 0.001) {
    activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_speed", { speed: settings.playbackSpeed });
  }
  if (hwdecModeFromSnapshot(activeSnapshot.hwdec) !== settings.hwdecMode) {
    activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_hwdec", { mode: settings.hwdecMode });
  }
  if (activeSnapshot.videoFill !== settings.videoFill) {
    activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_video_fill", { enabled: settings.videoFill });
  }
  activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_loop_file", { enabled: settings.loopMode === "one" });
  return activeSnapshot;
}

export async function applyStoredMediaPlaybackSettingsToMpv(path: string, snapshot: MpvSnapshot) {
  try {
    const mediaSettings = await invoke<MediaPlaybackSettings>("playback_media_settings", { path });
    if (!mediaSettings.hasSubtitleTrackSelection) {
      return snapshot;
    }
    return await invoke<MpvSnapshot>("mpv_embed_select_track", { kind: "subtitle", trackId: mediaSettings.subtitleTrackId });
  } catch (error) {
    console.warn("Failed to apply media playback settings", error);
    return snapshot;
  }
}

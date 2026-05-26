import { useState } from "react";
import type { MediaPanelMode } from "../app/types";

export function usePlaybackPanelState() {
  const [isPlaylistOpen, setIsPlaylistOpen] = useState(false);
  const [mediaPanelMode, setMediaPanelMode] = useState<MediaPanelMode | null>(null);

  function closeFloatingPlaybackMenus() {
    setMediaPanelMode(null);
    setIsPlaylistOpen(false);
  }

  function togglePlaylist() {
    setMediaPanelMode(null);
    setIsPlaylistOpen((isOpen) => !isOpen);
  }

  function toggleSpeedPanel() {
    setIsPlaylistOpen(false);
    setMediaPanelMode((mode) => (mode === "speed" ? null : "speed"));
  }

  function toggleTrackPanel() {
    setIsPlaylistOpen(false);
    setMediaPanelMode((mode) => (mode === "tracks" ? null : "tracks"));
  }

  function toggleLoopPanel() {
    setIsPlaylistOpen(false);
    setMediaPanelMode((mode) => (mode === "loop" ? null : "loop"));
  }

  return {
    isPlaylistOpen,
    setIsPlaylistOpen,
    mediaPanelMode,
    setMediaPanelMode,
    closeFloatingPlaybackMenus,
    togglePlaylist,
    toggleSpeedPanel,
    toggleTrackPanel,
    toggleLoopPanel,
  };
}

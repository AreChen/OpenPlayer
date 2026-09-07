import { runWindowCommand } from "../../app/windowControls";
import { usePlayerInteractionRuntime } from "../../hooks/usePlayerInteractionRuntime";
import { usePlayerOverlayFoundation } from "../../hooks/usePlayerOverlayFoundation";
import { usePlayerOverlayState } from "../../hooks/usePlayerOverlayState";
import { usePlayerPlaybackCoordinator } from "../../hooks/usePlayerPlaybackCoordinator";
import { usePlayerWorkspaceDomains } from "../../hooks/usePlayerWorkspaceDomains";
import { PlayerAppView } from "./PlayerAppView";
import { PlaybackClockContext } from "./PlaybackClockContext";
import { buildPlayerOverlayViewPropsFromDomains } from "./playerOverlayDomainViewProps";

export function PlayerOverlayApp() {
  const playerState = usePlayerOverlayState();
  const media = playerState.currentIndex === null ? null : (playerState.queue[playerState.currentIndex] ?? null);
  const foundation = usePlayerOverlayFoundation({
    playerPreferences: playerState.playerPreferences,
    platformSupport: playerState.platformSupport,
    previousAudibleVolumeRef: playerState.previousAudibleVolumeRef,
    hardwareDecodingModeRef: playerState.hardwareDecodingModeRef,
    setVolumeLevel: playerState.setVolumeLevel,
    setPlaybackSpeedValue: playerState.setPlaybackSpeedValue,
    setHardwareDecodingModeValue: playerState.setHardwareDecodingModeValue,
    setIsVideoFillEnabled: playerState.setIsVideoFillEnabled,
    setTimeDisplayModeValue: playerState.setTimeDisplayModeValue,
    setLoopModeValue: playerState.setLoopModeValue,
  });
  const playback = usePlayerPlaybackCoordinator({
    media,
    state: playerState,
    foundation,
  });
  const workspaceDomains = usePlayerWorkspaceDomains({
    media,
    state: playerState,
    foundation,
    playback,
  });
  const interactionRuntime = usePlayerInteractionRuntime({
    media,
    state: playerState,
    foundation,
    playback,
    workspace: workspaceDomains,
  });

  const viewProps = buildPlayerOverlayViewPropsFromDomains({
    media,
    state: playerState,
    foundation,
    playback,
    workspace: workspaceDomains,
    interaction: interactionRuntime,
    onWindowCommand: runWindowCommand,
  });

  return (
    <PlaybackClockContext.Provider value={{
      clock: playback.clock,
      timelineVisible: interactionRuntime.chrome.isChromeVisible || interactionRuntime.chrome.isChromePinned,
      framesPerSecond: playerState.framesPerSecond,
      locale: foundation.locale,
    }}>
      <PlayerAppView {...viewProps} />
    </PlaybackClockContext.Provider>
  );
}

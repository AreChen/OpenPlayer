import { Icon } from "../../app/Icon";
import { formatPlaybackSpeed } from "../../app/playback";
import { PluginActionButton } from "../plugins/PluginActionButton";
import type { TransportControlsProps } from "./TransportControls.types";

type TransportControlStripProps = Pick<
  TransportControlsProps,
  | "t"
  | "locale"
  | "mediaLoaded"
  | "isPickerOpen"
  | "isPlaying"
  | "mediaPanelMode"
  | "isMuted"
  | "volumeMuteLabel"
  | "volumeLevel"
  | "playbackSpeed"
  | "isPlaylistOpen"
  | "hardwareDecodingMode"
  | "hardwareDecodingLabel"
  | "hardwareDecodingToggleLabel"
  | "pluginControlLeftActions"
  | "pluginControlCenterActions"
  | "pluginControlRightActions"
  | "isPluginActionDisabled"
  | "onExecutePluginAction"
  | "onOpenNativeMediaFiles"
  | "onStopPlayback"
  | "onTogglePlayback"
  | "onToggleLoopPanel"
  | "onToggleMute"
  | "onSetVolume"
  | "onToggleSpeedPanel"
  | "onToggleTrackPanel"
  | "onTogglePlaylist"
  | "onToggleHardwareDecoding"
>;

export function TransportControlStrip({
  t,
  locale,
  mediaLoaded,
  isPickerOpen,
  isPlaying,
  mediaPanelMode,
  isMuted,
  volumeMuteLabel,
  volumeLevel,
  playbackSpeed,
  isPlaylistOpen,
  hardwareDecodingMode,
  hardwareDecodingLabel,
  hardwareDecodingToggleLabel,
  pluginControlLeftActions,
  pluginControlCenterActions,
  pluginControlRightActions,
  isPluginActionDisabled,
  onExecutePluginAction,
  onOpenNativeMediaFiles,
  onStopPlayback,
  onTogglePlayback,
  onToggleLoopPanel,
  onToggleMute,
  onSetVolume,
  onToggleSpeedPanel,
  onToggleTrackPanel,
  onTogglePlaylist,
  onToggleHardwareDecoding,
}: TransportControlStripProps) {
  return (
    <div className="control-strip">
      <button className="open-media-button" type="button" aria-label={t.controls.openMedia} onClick={onOpenNativeMediaFiles} disabled={isPickerOpen}>
        <Icon name="folder" />
      </button>
      {pluginControlLeftActions.map((instance) => (
        <PluginActionButton
          key={`${instance.plugin.id}:${instance.action.id}`}
          instance={instance}
          compact
          locale={locale}
          t={t}
          disabled={isPluginActionDisabled(instance.action)}
          onExecute={onExecutePluginAction}
        />
      ))}
      <button type="button" aria-label={t.controls.stop} onClick={onStopPlayback} disabled={!mediaLoaded}>
        <Icon name="stop" />
      </button>
      <button
        className="control-primary"
        type="button"
        aria-label={isPlaying ? t.controls.pause : mediaLoaded ? t.controls.play : t.controls.openMedia}
        onClick={onTogglePlayback}
        disabled={!mediaLoaded && isPickerOpen}
      >
        <Icon name={isPlaying ? "pause" : "play"} />
      </button>
      {pluginControlCenterActions.map((instance) => (
        <PluginActionButton
          key={`${instance.plugin.id}:${instance.action.id}`}
          instance={instance}
          compact
          locale={locale}
          t={t}
          disabled={isPluginActionDisabled(instance.action)}
          onExecute={onExecutePluginAction}
        />
      ))}
      <button
        className={mediaPanelMode === "loop" ? "loop-toggle loop-toggle--open" : "loop-toggle"}
        type="button"
        aria-label={t.controls.openLoopMode}
        aria-expanded={mediaPanelMode === "loop"}
        onClick={onToggleLoopPanel}
        disabled={!mediaLoaded}
      >
        <Icon name="restart" />
      </button>
      <div className={isMuted ? "volume-control volume-control--muted" : "volume-control"} aria-label={t.controls.volume}>
        <button className="volume-mute-button" type="button" aria-label={volumeMuteLabel} aria-pressed={isMuted} onClick={onToggleMute}>
          <Icon name={isMuted ? "volumeMuted" : "volume"} />
        </button>
        <input type="range" min="0" max="1" step="0.01" value={volumeLevel} aria-label={t.controls.volume} onChange={(event) => onSetVolume(Number(event.currentTarget.value))} />
      </div>
      <button className="speed-toggle" type="button" aria-label={t.controls.openPlaybackSpeed} aria-expanded={mediaPanelMode === "speed"} onClick={onToggleSpeedPanel} disabled={!mediaLoaded}>
        {formatPlaybackSpeed(playbackSpeed)}
      </button>
      <button
        className={`tracks-toggle ${mediaPanelMode === "tracks" ? "tracks-toggle--open" : ""}`}
        type="button"
        aria-label={t.controls.openTracks}
        aria-expanded={mediaPanelMode === "tracks"}
        onClick={onToggleTrackPanel}
        disabled={!mediaLoaded}
      >
        <Icon name="tracks" />
      </button>
      <button
        className={`playlist-toggle ${isPlaylistOpen ? "playlist-toggle--open" : ""}`}
        type="button"
        aria-label={t.controls.togglePlaylist}
        aria-expanded={isPlaylistOpen}
        onClick={onTogglePlaylist}
      >
        <Icon name="list" />
      </button>
      {pluginControlRightActions.map((instance) => (
        <PluginActionButton
          key={`${instance.plugin.id}:${instance.action.id}`}
          instance={instance}
          compact
          locale={locale}
          t={t}
          disabled={isPluginActionDisabled(instance.action)}
          onExecute={onExecutePluginAction}
        />
      ))}
      <button
        className={`decode-toggle decode-toggle--${hardwareDecodingMode}`}
        type="button"
        aria-label={hardwareDecodingToggleLabel}
        aria-pressed={hardwareDecodingMode === "hardware"}
        title={hardwareDecodingToggleLabel}
        onClick={onToggleHardwareDecoding}
      >
        <Icon name="cpu" />
        <span>{hardwareDecodingLabel}</span>
      </button>
    </div>
  );
}

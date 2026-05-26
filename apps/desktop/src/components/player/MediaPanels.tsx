import { SUBTITLE_DELAY_STEP_SECONDS, playbackSpeedOptions } from "../../app/constants";
import { formatPlaybackSpeed, formatSubtitleDelay, loopModeLabel } from "../../app/playback";
import type { LoopMode, MpvTrack, PluginSettingDefinition, PluginSettingValue, SelectableTrackKind, ThemePluginSummary } from "../../app/types";
import type { AppStrings } from "../../i18n";
import { TrackList, VideoLayoutOptions } from "../media/MediaPanelSections";
import { PluginSettingControl } from "../plugins/PluginSettingControl";

type SpeedPanelProps = {
  t: AppStrings;
  playbackSpeed: number;
  onSetPlaybackSpeed: (speed: number) => void;
};

export function SpeedPanel({ t, playbackSpeed, onSetPlaybackSpeed }: SpeedPanelProps) {
  return (
    <aside className="media-panel media-panel--speed" aria-label={t.media.speed} onContextMenu={(event) => event.stopPropagation()} onPointerDown={(event) => event.stopPropagation()}>
      <section className="media-panel-section">
        <header>
          <h3>{t.media.speed}</h3>
          <span>{formatPlaybackSpeed(playbackSpeed)}</span>
        </header>
        <div className="speed-options" role="group" aria-label={t.media.speed}>
          {playbackSpeedOptions.map((speed) => (
            <button
              key={speed}
              className={Math.abs(playbackSpeed - speed) < 0.001 ? "speed-option speed-option--active" : "speed-option"}
              type="button"
              aria-pressed={Math.abs(playbackSpeed - speed) < 0.001}
              onClick={() => onSetPlaybackSpeed(speed)}
            >
              {formatPlaybackSpeed(speed)}
            </button>
          ))}
        </div>
      </section>
    </aside>
  );
}

type LoopPanelProps = {
  t: AppStrings;
  loopMode: LoopMode;
  loopModeOptions: Array<{ mode: LoopMode; label: string; description: string }>;
  onSetLoopMode: (mode: LoopMode) => void;
};

export function LoopPanel({ t, loopMode, loopModeOptions, onSetLoopMode }: LoopPanelProps) {
  return (
    <aside className="media-panel media-panel--loop" aria-label={t.media.loopMode} onContextMenu={(event) => event.stopPropagation()} onPointerDown={(event) => event.stopPropagation()}>
      <section className="media-panel-section">
        <header>
          <h3>{t.media.loopMode}</h3>
          <span>{loopModeLabel(loopMode, t)}</span>
        </header>
        <div className="loop-options" role="group" aria-label={t.media.loopMode}>
          {loopModeOptions.map((option) => (
            <button
              key={option.mode}
              className={loopMode === option.mode ? "loop-option loop-option--active" : "loop-option"}
              type="button"
              aria-pressed={loopMode === option.mode}
              onClick={() => onSetLoopMode(option.mode)}
            >
              <span>{option.label}</span>
              <small>{option.description}</small>
            </button>
          ))}
        </div>
      </section>
    </aside>
  );
}

type SubtitlePluginSettingGroup = {
  plugin: ThemePluginSummary;
  settings: PluginSettingDefinition[];
};

type TracksPanelProps = {
  t: AppStrings;
  locale: string;
  isAudioOnlyMedia: boolean;
  isVideoFillEnabled: boolean;
  audioTracks: MpvTrack[];
  videoTracks: MpvTrack[];
  subtitleTracks: MpvTrack[];
  subtitleDelay: number;
  subtitlePluginSettingGroups: SubtitlePluginSettingGroup[];
  isPickerOpen: boolean;
  systemFontFamilies: string[];
  onSetVideoFillMode: (enabled: boolean) => void;
  onSelectTrack: (kind: SelectableTrackKind, trackId: number | null) => void;
  onSetSubtitleDelay: (delay: number) => void;
  onSetPluginSettingValue: (pluginId: string, setting: PluginSettingDefinition, value: PluginSettingValue) => void;
  onChoosePluginDirectory: (pluginId: string, setting: PluginSettingDefinition) => void;
  onOpenPluginDirectory: (setting: PluginSettingDefinition) => void;
  onAddExternalSubtitle: () => void;
};

export function TracksPanel({
  t,
  locale,
  isAudioOnlyMedia,
  isVideoFillEnabled,
  audioTracks,
  videoTracks,
  subtitleTracks,
  subtitleDelay,
  subtitlePluginSettingGroups,
  isPickerOpen,
  systemFontFamilies,
  onSetVideoFillMode,
  onSelectTrack,
  onSetSubtitleDelay,
  onSetPluginSettingValue,
  onChoosePluginDirectory,
  onOpenPluginDirectory,
  onAddExternalSubtitle,
}: TracksPanelProps) {
  return (
    <aside className="media-panel media-panel--tracks" aria-label={t.contextMenu.tracksSubtitles} onContextMenu={(event) => event.stopPropagation()} onPointerDown={(event) => event.stopPropagation()}>
      <VideoLayoutOptions isAudioOnlyMedia={isAudioOnlyMedia} isVideoFillEnabled={isVideoFillEnabled} t={t} onSetVideoFillMode={onSetVideoFillMode} />
      <TrackList kind="audio" label={t.media.audioTracks} items={audioTracks} t={t} onSelectTrack={onSelectTrack} />
      <TrackList kind="video" label={t.media.videoTracks} items={videoTracks} t={t} onSelectTrack={onSelectTrack} />
      <TrackList kind="subtitle" label={t.media.subtitles} items={subtitleTracks} t={t} onSelectTrack={onSelectTrack} />

      <section className="media-panel-section subtitle-delay">
        <header>
          <h3>{t.media.subtitleSync}</h3>
          <span>{formatSubtitleDelay(subtitleDelay)}</span>
        </header>
        <div className="subtitle-delay-controls">
          <button type="button" onClick={() => onSetSubtitleDelay(subtitleDelay - SUBTITLE_DELAY_STEP_SECONDS)}>
            -0.1s
          </button>
          <output>{formatSubtitleDelay(subtitleDelay)}</output>
          <button type="button" onClick={() => onSetSubtitleDelay(subtitleDelay + SUBTITLE_DELAY_STEP_SECONDS)}>
            +0.1s
          </button>
          <button type="button" onClick={() => onSetSubtitleDelay(0)} disabled={Math.abs(subtitleDelay) < 0.005}>
            {t.common.reset}
          </button>
        </div>
      </section>

      {subtitlePluginSettingGroups.map((group) => (
        <section className="media-panel-section plugin-slot-section" key={group.plugin.id}>
          <header>
            <h3>{group.plugin.name}</h3>
            <span>{t.settings.plugins.slot}</span>
          </header>
          <div className="plugin-slot-controls">
            {group.settings.map((setting) => (
              <PluginSettingControl
                key={`${group.plugin.id}:${setting.id}:compact`}
                plugin={group.plugin}
                setting={setting}
                compact
                locale={locale}
                t={t}
                isPickerOpen={isPickerOpen}
                systemFontFamilies={systemFontFamilies}
                onValueChange={onSetPluginSettingValue}
                onChooseDirectory={onChoosePluginDirectory}
                onOpenDirectory={onOpenPluginDirectory}
              />
            ))}
          </div>
        </section>
      ))}

      <button className="subtitle-load" type="button" onClick={onAddExternalSubtitle} disabled={isPickerOpen}>
        {t.media.loadExternalSubtitle}
      </button>
    </aside>
  );
}

import { Icon } from "../../app/Icon";
import { audioVisualizerBarLevels, openPlayerLogoUrl } from "../../app/constants";
import { formatTimecode, platformUnsupportedPlaybackMessage } from "../../app/playback";
import type {
  AlwaysOnTopFeedback,
  CaptureFeedback,
  MediaItem,
  MpvRecordingState,
  MpvTrack,
  PlatformSupport,
  ResizeFeedback,
  ThemeStyleProperties,
  VolumeFeedback,
  WindowCommand,
} from "../../app/types";
import { resizeDirectionClassName } from "../../app/windowControls";
import type { AppStrings } from "../../i18n";

type StageOverlaysProps = {
  t: AppStrings;
  media: MediaItem | null;
  platformSupport: PlatformSupport | null;
  isAudioOnlyMedia: boolean;
  isPlaying: boolean;
  primaryAudioTrack: MpvTrack | null;
  displayTime: number;
  duration: number;
  resizeFeedback: ResizeFeedback | null;
  isDropActive: boolean;
  playbackError: string | null;
  volumeFeedback: VolumeFeedback | null;
  alwaysOnTopFeedback: AlwaysOnTopFeedback | null;
  captureFeedback: CaptureFeedback | null;
  recordingState: MpvRecordingState;
  onWindowCommand: (command: WindowCommand) => void | Promise<void>;
};

export function StageOverlays({
  t,
  media,
  platformSupport,
  isAudioOnlyMedia,
  isPlaying,
  primaryAudioTrack,
  displayTime,
  duration,
  resizeFeedback,
  isDropActive,
  playbackError,
  volumeFeedback,
  alwaysOnTopFeedback,
  captureFeedback,
  recordingState,
  onWindowCommand,
}: StageOverlaysProps) {
  return (
    <>
      {!media && (
        <div className="empty-open">
          <img className="empty-open-logo" src={openPlayerLogoUrl} alt="" draggable={false} />
          <span>{t.contextMenu.openMedia}</span>
          {platformSupport && !platformSupport.mpvEmbedVideo && <small className="platform-support-note">{platformUnsupportedPlaybackMessage(platformSupport, t)}</small>}
        </div>
      )}

      {isAudioOnlyMedia && media && (
        <div className={isPlaying ? "audio-visualizer" : "audio-visualizer audio-visualizer--paused"} aria-hidden="true">
          <div className="audio-visualizer-bars">
            {audioVisualizerBarLevels.map((level, index) => (
              <span key={index} style={{ "--bar-level": String(level), "--bar-delay": `${index * -86}ms` } as ThemeStyleProperties} />
            ))}
          </div>
          <div className="audio-visualizer-grid">
            <div className="audio-visualizer-copy">
              <span>{media.name}</span>
              <small>
                {(primaryAudioTrack?.codec ?? "audio").toUpperCase()} · {formatTimecode(displayTime, duration)}
              </small>
            </div>
          </div>
        </div>
      )}

      {resizeFeedback && (
        <div aria-hidden="true" className={`resize-feedback resize-feedback--${resizeDirectionClassName(resizeFeedback.direction)} ${resizeFeedback.active ? "resize-feedback--active" : ""}`}>
          <span className="resize-feedback-line resize-feedback-line--north" />
          <span className="resize-feedback-line resize-feedback-line--south" />
          <span className="resize-feedback-line resize-feedback-line--east" />
          <span className="resize-feedback-line resize-feedback-line--west" />
          <span className="resize-feedback-corner resize-feedback-corner--north-east" />
          <span className="resize-feedback-corner resize-feedback-corner--north-west" />
          <span className="resize-feedback-corner resize-feedback-corner--south-east" />
          <span className="resize-feedback-corner resize-feedback-corner--south-west" />
        </div>
      )}

      <div className="window-controls" aria-label={t.contextMenu.closeWindow}>
        <button type="button" aria-label={t.controls.minimize} onClick={() => onWindowCommand("window_minimize")}>
          <Icon name="minimize" />
        </button>
        <button type="button" aria-label={t.controls.maximize} onClick={() => onWindowCommand("window_toggle_maximize")}>
          <Icon name="maximize" />
        </button>
        <button className="window-control-close" type="button" aria-label={t.controls.close} onClick={() => onWindowCommand("window_close")}>
          <Icon name="close" />
        </button>
      </div>

      {isDropActive && (
        <div className="drop-overlay" aria-live="polite">
          <Icon name="folderAdd" />
          <span>{t.media.dropToPlay}</span>
        </div>
      )}

      {playbackError && (
        <div className="playback-error" role="alert">
          {playbackError}
        </div>
      )}
      {volumeFeedback && (
        <div className="volume-feedback" role="status" aria-live="polite">
          <Icon name="volume" />
          <span>{Math.round(volumeFeedback.level * 100)}%</span>
        </div>
      )}
      {alwaysOnTopFeedback && (
        <div className="volume-feedback always-on-top-feedback" role="status" aria-live="polite">
          <Icon name="pin" />
          <span>{alwaysOnTopFeedback.enabled ? t.status.alwaysOnTopEnabled : t.status.alwaysOnTopDisabled}</span>
        </div>
      )}
      {captureFeedback && (
        <div className="volume-feedback capture-feedback" role="status" aria-live="polite">
          <Icon name={captureFeedback.icon} />
          <span>{captureFeedback.message}</span>
        </div>
      )}
      {recordingState.active && (
        <div className="recording-indicator" role="status" aria-live="polite">
          <Icon name="record" />
          <span>{t.status.recordingActive}</span>
        </div>
      )}
    </>
  );
}

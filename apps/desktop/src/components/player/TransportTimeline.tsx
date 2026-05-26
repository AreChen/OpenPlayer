import type { CSSProperties } from "react";
import { Icon } from "../../app/Icon";
import type { TransportControlsProps } from "./TransportControls.types";

type TransportTimelineProps = Pick<
  TransportControlsProps,
  | "t"
  | "mediaLoaded"
  | "duration"
  | "displayTime"
  | "progress"
  | "progressRatio"
  | "effectiveTimeDisplayMode"
  | "canShowFrames"
  | "currentTransportLabel"
  | "durationTransportLabel"
  | "currentTimeToggleLabel"
  | "durationTimeToggleLabel"
  | "previousIndex"
  | "nextIndex"
  | "onToggleTimeDisplayMode"
  | "onPlayPreviousQueueItem"
  | "onSeekTo"
  | "onCommitSeekTo"
  | "onPlayNextQueueItem"
>;

export function TransportTimeline({
  t,
  mediaLoaded,
  duration,
  displayTime,
  progress,
  progressRatio,
  effectiveTimeDisplayMode,
  canShowFrames,
  currentTransportLabel,
  durationTransportLabel,
  currentTimeToggleLabel,
  durationTimeToggleLabel,
  previousIndex,
  nextIndex,
  onToggleTimeDisplayMode,
  onPlayPreviousQueueItem,
  onSeekTo,
  onCommitSeekTo,
  onPlayNextQueueItem,
}: TransportTimelineProps) {
  return (
    <div className="transport-row">
      <button
        className="transport-time transport-time--toggle"
        type="button"
        aria-label={currentTimeToggleLabel}
        aria-pressed={effectiveTimeDisplayMode === "frames"}
        onClick={onToggleTimeDisplayMode}
        disabled={!canShowFrames}
      >
        {currentTransportLabel}
      </button>
      <button
        className="timeline-step-button"
        type="button"
        aria-label={t.controls.previousVideo}
        onClick={onPlayPreviousQueueItem}
        disabled={previousIndex === null}
      >
        <Icon name="previous" />
      </button>
      <div
        className="seek-control"
        style={{ "--progress": `${progress}%`, "--progress-ratio": progressRatio } as CSSProperties}
      >
        <div className="seek-rail" aria-hidden="true">
          <div className="seek-progress" />
        </div>
        <div className="seek-thumb" aria-hidden="true" />
        <input
          className="seek-slider"
          type="range"
          min="0"
          max={duration || 0}
          step="any"
          value={displayTime}
          aria-label={t.controls.seek}
          onChange={(event) => onSeekTo(Number(event.currentTarget.value))}
          onPointerUp={(event) => onCommitSeekTo(Number(event.currentTarget.value))}
          onKeyUp={(event) => onCommitSeekTo(Number(event.currentTarget.value))}
          onBlur={(event) => onCommitSeekTo(Number(event.currentTarget.value))}
          disabled={!mediaLoaded || duration <= 0}
        />
      </div>
      <button
        className="timeline-step-button"
        type="button"
        aria-label={t.controls.nextVideo}
        onClick={onPlayNextQueueItem}
        disabled={nextIndex === null}
      >
        <Icon name="next" />
      </button>
      <button
        className="transport-time transport-time--toggle"
        type="button"
        aria-label={durationTimeToggleLabel}
        aria-pressed={effectiveTimeDisplayMode === "frames"}
        onClick={onToggleTimeDisplayMode}
        disabled={!canShowFrames}
      >
        {durationTransportLabel}
      </button>
    </div>
  );
}

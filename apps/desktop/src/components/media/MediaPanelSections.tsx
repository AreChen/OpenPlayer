import { trackDisplayLabel } from "../../app/playback";
import type { MpvTrack, SelectableTrackKind } from "../../app/types";
import type { AppStrings } from "../../i18n";

type TrackListProps = {
  kind: SelectableTrackKind;
  label: string;
  items: MpvTrack[];
  t: AppStrings;
  onSelectTrack: (kind: SelectableTrackKind, trackId: number | null) => void;
};

export function TrackList({ kind, label, items, t, onSelectTrack }: TrackListProps) {
  const hasSelected = items.some((track) => track.selected);

  return (
    <section className="media-panel-section">
      <header>
        <h3>{label}</h3>
        <span>{t.media.trackCount(items.length)}</span>
      </header>
      <div className="track-list">
        {kind === "subtitle" && (
          <button className={`track-item ${hasSelected ? "" : "track-item--active"}`} type="button" onClick={() => onSelectTrack(kind, null)}>
            <span>{t.media.closeSubtitles}</span>
            <small>{t.common.off}</small>
          </button>
        )}
        {items.map((track) => (
          <button
            key={`${track.kind}:${track.id}`}
            className={`track-item ${track.selected ? "track-item--active" : ""}`}
            type="button"
            onClick={() => onSelectTrack(kind, track.id)}
          >
            <span>{trackDisplayLabel(track, t)}</span>
            <small>ID {track.id}</small>
          </button>
        ))}
        {!items.length && kind !== "subtitle" && <div className="track-empty">{t.media.noSwitchableTracks}</div>}
      </div>
    </section>
  );
}

type VideoLayoutOptionsProps = {
  isAudioOnlyMedia: boolean;
  isVideoFillEnabled: boolean;
  t: AppStrings;
  onSetVideoFillMode: (enabled: boolean) => void;
};

export function VideoLayoutOptions({ isAudioOnlyMedia, isVideoFillEnabled, t, onSetVideoFillMode }: VideoLayoutOptionsProps) {
  if (isAudioOnlyMedia) {
    return null;
  }

  return (
    <section className="media-panel-section video-layout">
      <header>
        <h3>{t.media.videoLayout}</h3>
        <span>{isVideoFillEnabled ? t.media.videoFill : t.media.videoFit}</span>
      </header>
      <div className="video-layout-options">
        <button
          className={isVideoFillEnabled ? "video-layout-option" : "video-layout-option video-layout-option--active"}
          type="button"
          onClick={() => onSetVideoFillMode(false)}
        >
          <span>{t.media.videoFit}</span>
          <small>{t.media.videoFitDescription}</small>
        </button>
        <button
          className={isVideoFillEnabled ? "video-layout-option video-layout-option--active" : "video-layout-option"}
          type="button"
          onClick={() => onSetVideoFillMode(true)}
        >
          <span>{t.media.videoFill}</span>
          <small>{t.media.videoFillDescription}</small>
        </button>
      </div>
    </section>
  );
}

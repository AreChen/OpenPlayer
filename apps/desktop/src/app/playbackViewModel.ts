import type { AppLocale, AppStrings } from "../i18n";
import { isAudioMediaPath } from "./media";
import { canDisplayFrames, formatFrameCount, formatTimecode } from "./playback";
import type {
  AppearanceState,
  HardwareDecodingMode,
  MediaItem,
  MpvTrack,
  TimeDisplayMode,
} from "./types";

type PlaybackViewModelInput = {
  t: AppStrings;
  locale: AppLocale;
  queue: MediaItem[];
  media: MediaItem | null;
  loadedMediaPath: string | null;
  tracks: MpvTrack[];
  displayTime: number;
  duration: number;
  isPlaying: boolean;
  framesPerSecond: number;
  timeDisplayMode: TimeDisplayMode;
  volumeLevel: number;
  hardwareDecodingMode: HardwareDecodingMode;
  appearanceState: AppearanceState | null;
  isChromeVisible: boolean;
  isChromePinned: boolean;
};

export function buildPlaybackViewModel({
  t,
  locale,
  queue,
  media,
  loadedMediaPath,
  tracks,
  displayTime,
  duration,
  isPlaying,
  framesPerSecond,
  timeDisplayMode,
  volumeLevel,
  hardwareDecodingMode,
  appearanceState,
  isChromeVisible,
  isChromePinned,
}: PlaybackViewModelInput) {
  const progress = duration > 0 ? Math.min(100, Math.max(0, (displayTime / duration) * 100)) : 0;
  const audioTracks = tracks.filter((track) => track.kind === "audio");
  const videoTracks = tracks.filter((track) => track.kind === "video");
  const subtitleTracks = tracks.filter((track) => track.kind === "sub");
  const isAudioOnlyMedia = Boolean(media && loadedMediaPath === media.path && isAudioMediaPath(media.path));
  const primaryAudioTrack = audioTracks.find((track) => track.selected) ?? audioTracks[0] ?? null;
  const subtitlePluginSettingGroups = (appearanceState?.plugins ?? [])
    .filter((plugin) => plugin.enabled)
    .map((plugin) => ({
      plugin,
      settings: plugin.settings.filter((setting) => setting.placement === "subtitleSettings"),
    }))
    .filter((group) => group.settings.length > 0);
  const canShowFrames = canDisplayFrames(framesPerSecond, duration);
  const effectiveTimeDisplayMode: TimeDisplayMode = timeDisplayMode === "frames" && canShowFrames ? "frames" : "timecode";
  const totalFrames = canShowFrames ? Math.max(0, Math.floor(duration * framesPerSecond)) : 0;
  const currentFrame = canShowFrames ? Math.min(totalFrames, Math.max(0, Math.floor(displayTime * framesPerSecond))) : 0;

  return {
    progress,
    progressRatio: progress / 100,
    queueItems: queue.length ? queue : media ? [media] : [],
    audioTracks,
    videoTracks,
    subtitleTracks,
    isAudioOnlyMedia,
    primaryAudioTrack,
    subtitlePluginSettingGroups,
    canShowFrames,
    effectiveTimeDisplayMode,
    currentTransportLabel: effectiveTimeDisplayMode === "frames" ? formatFrameCount(currentFrame, locale) : formatTimecode(displayTime, duration),
    durationTransportLabel: effectiveTimeDisplayMode === "frames" ? formatFrameCount(totalFrames, locale) : formatTimecode(duration, duration),
    isMuted: volumeLevel <= 0,
    volumeMuteLabel: volumeLevel <= 0 ? t.controls.unmute : t.controls.mute,
    currentTimeToggleLabel: t.controls.currentTime,
    durationTimeToggleLabel: t.controls.duration,
    isChromeHidden: Boolean(media) && !isChromeVisible && !isChromePinned,
    hardwareDecodingLabel: hardwareDecodingMode === "hardware" ? t.hardware.hardware : t.hardware.software,
    hardwareDecodingToggleLabel: hardwareDecodingMode === "hardware" ? t.hardware.switchToSoftware : t.hardware.switchToHardware,
  };
}

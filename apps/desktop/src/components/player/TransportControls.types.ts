import type {
  HardwareDecodingMode,
  MediaPanelMode,
  PluginActionDefinition,
  PluginActionInstance,
  TimeDisplayMode,
} from "../../app/types";
import type { AppStrings } from "../../i18n";

export type TransportControlsProps = {
  t: AppStrings;
  locale: string;
  mediaLoaded: boolean;
  duration: number;
  displayTime: number;
  progress: number;
  progressRatio: number;
  effectiveTimeDisplayMode: TimeDisplayMode;
  canShowFrames: boolean;
  currentTransportLabel: string;
  durationTransportLabel: string;
  currentTimeToggleLabel: string;
  durationTimeToggleLabel: string;
  previousIndex: number | null;
  nextIndex: number | null;
  isPickerOpen: boolean;
  isPlaying: boolean;
  mediaPanelMode: MediaPanelMode | null;
  isMuted: boolean;
  volumeMuteLabel: string;
  volumeLevel: number;
  playbackSpeed: number;
  isPlaylistOpen: boolean;
  hardwareDecodingMode: HardwareDecodingMode;
  hardwareDecodingLabel: string;
  hardwareDecodingToggleLabel: string;
  pluginControlLeftActions: PluginActionInstance[];
  pluginControlCenterActions: PluginActionInstance[];
  pluginControlRightActions: PluginActionInstance[];
  isPluginActionDisabled: (action: PluginActionDefinition) => boolean;
  onExecutePluginAction: (instance: PluginActionInstance) => void;
  onToggleTimeDisplayMode: () => void;
  onPlayPreviousQueueItem: () => void;
  onSeekTo: (value: number) => void;
  onCommitSeekTo: (value: number) => void;
  onPlayNextQueueItem: () => void;
  onOpenNativeMediaFiles: () => void;
  onStopPlayback: () => void;
  onTogglePlayback: () => void;
  onToggleLoopPanel: () => void;
  onToggleMute: () => void;
  onSetVolume: (value: number) => void;
  onToggleSpeedPanel: () => void;
  onToggleTrackPanel: () => void;
  onTogglePlaylist: () => void;
  onToggleHardwareDecoding: () => void;
};

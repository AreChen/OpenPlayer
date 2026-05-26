import type { MouseEvent as ReactMouseEvent, MutableRefObject } from "react";
import type {
  ActivePluginView,
  ContextMenuPosition,
  MediaItem,
  MediaPanelMode,
  ShortcutAction,
} from "../app/types";
import { useAppShellHandlers } from "./useAppShellHandlers";
import { useChromeAutoHide } from "./useChromeAutoHide";
import { useWindowFrameInteractions } from "./useWindowFrameInteractions";

type UsePlayerChromeInteractionsOptions = {
  media: MediaItem | null;
  mediaPanelMode: MediaPanelMode | null;
  isPlaylistOpen: boolean;
  isPickerOpen: boolean;
  playbackError: string | null;
  contextMenu: ContextMenuPosition | null;
  isSettingsOpen: boolean;
  isNetworkStreamDialogOpen: boolean;
  activePluginView: ActivePluginView | null;
  quietKeyboardControls: boolean;
  clearResizeHoverFeedbackRef: MutableRefObject<() => void>;
  platformOs: string | null | undefined;
  recordingShortcutAction: ShortcutAction | null;
  volumeLevel: number;
  openContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
  closeContextMenu: () => void;
  closeFloatingPlaybackMenus: () => void;
  togglePlayback: () => void;
  setVolume: (value: number, options?: { feedback?: boolean }) => void;
};

export function usePlayerChromeInteractions({
  media,
  mediaPanelMode,
  isPlaylistOpen,
  isPickerOpen,
  playbackError,
  contextMenu,
  isSettingsOpen,
  isNetworkStreamDialogOpen,
  activePluginView,
  quietKeyboardControls,
  clearResizeHoverFeedbackRef,
  platformOs,
  recordingShortcutAction,
  volumeLevel,
  openContextMenu,
  closeContextMenu,
  closeFloatingPlaybackMenus,
  togglePlayback,
  setVolume,
}: UsePlayerChromeInteractionsOptions) {
  const isMediaPanelOpen = mediaPanelMode !== null;
  const isChromePinned =
    !media ||
    isPlaylistOpen ||
    isMediaPanelOpen ||
    isPickerOpen ||
    playbackError !== null ||
    contextMenu !== null ||
    isSettingsOpen ||
    isNetworkStreamDialogOpen ||
    activePluginView !== null;
  const { isChromeVisible, recordUserActivity, recordShortcutActivity, handleShellPointerLeave } = useChromeAutoHide({
    mediaId: media?.id,
    isChromePinned,
    quietKeyboardControls,
    onPointerExit: () => clearResizeHoverFeedbackRef.current(),
  });
  const {
    resizeFeedback,
    handleDragRegionDoubleClick,
    handleDragRegionPointerDown,
    handleDragRegionPointerMove,
    handleDragRegionPointerEnd,
    handleResizePointerEnter,
    handleResizePointerLeave,
    handleResizePointerDown,
    handleResizePointerMove,
    handleResizePointerEnd,
    clearResizeHoverFeedback,
    clearWindowFrameInteraction,
  } = useWindowFrameInteractions({
    platformOs,
    onTogglePlayback: togglePlayback,
    onUserActivity: recordUserActivity,
  });
  clearResizeHoverFeedbackRef.current = clearResizeHoverFeedback;
  const shellHandlers = useAppShellHandlers({
    contextMenu,
    isSettingsOpen,
    isNetworkStreamDialogOpen,
    recordingShortcutAction,
    volumeLevel,
    recordUserActivity,
    openContextMenu,
    closeContextMenu,
    closeFloatingPlaybackMenus,
    handleShellPointerLeave,
    setVolume,
  });

  return {
    isChromePinned,
    isChromeVisible,
    recordUserActivity,
    recordShortcutActivity,
    resizeFeedback,
    handleDragRegionDoubleClick,
    handleDragRegionPointerDown,
    handleDragRegionPointerMove,
    handleDragRegionPointerEnd,
    handleResizePointerEnter,
    handleResizePointerLeave,
    handleResizePointerDown,
    handleResizePointerMove,
    handleResizePointerEnd,
    clearWindowFrameInteraction,
    shellHandlers,
  };
}

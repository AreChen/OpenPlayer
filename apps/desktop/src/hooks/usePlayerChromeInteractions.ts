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
  clearResizeHoverCursorRef: MutableRefObject<() => void>;
  platformOs: string | null | undefined;
  recordingShortcutAction: ShortcutAction | null;
  volumeLevel: number;
  openContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
  closeContextMenu: () => void;
  closePluginView: () => void;
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
  clearResizeHoverCursorRef,
  platformOs,
  recordingShortcutAction,
  volumeLevel,
  openContextMenu,
  closeContextMenu,
  closePluginView,
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
    onPointerExit: () => clearResizeHoverCursorRef.current(),
  });
  const {
    handleDragRegionDoubleClick,
    handleDragRegionPointerDown,
    handleDragRegionPointerMove,
    handleDragRegionPointerEnd,
    handleResizePointerEnter,
    handleResizePointerLeave,
    handleResizePointerDown,
    handleResizeSurfacePointerDown,
    handleResizeSurfacePointerMove,
    handleResizeSurfacePointerEnd,
    handleResizePointerMove,
    handleResizePointerEnd,
    clearResizeHoverCursor,
    clearWindowFrameInteraction,
  } = useWindowFrameInteractions({
    platformOs,
    onTogglePlayback: togglePlayback,
    onUserActivity: recordUserActivity,
  });
  clearResizeHoverCursorRef.current = clearResizeHoverCursor;
  const baseShellHandlers = useAppShellHandlers({
    contextMenu,
    activePluginView,
    isSettingsOpen,
    isNetworkStreamDialogOpen,
    recordingShortcutAction,
    volumeLevel,
    recordUserActivity,
    openContextMenu,
    closeContextMenu,
    closePluginView,
    closeFloatingPlaybackMenus,
    handleShellPointerLeave,
    setVolume,
  });
  const shellHandlers = {
    ...baseShellHandlers,
    onPointerDownCapture: handleResizeSurfacePointerDown,
    onPointerMoveCapture: handleResizeSurfacePointerMove,
    onPointerUpCapture: handleResizeSurfacePointerEnd,
    onPointerCancelCapture: handleResizeSurfacePointerEnd,
  };

  return {
    isChromePinned,
    isChromeVisible,
    recordUserActivity,
    recordShortcutActivity,
    handleDragRegionDoubleClick,
    handleDragRegionPointerDown,
    handleDragRegionPointerMove,
    handleDragRegionPointerEnd,
    handleResizePointerEnter,
    handleResizePointerLeave,
    handleResizePointerDown,
    handleResizeSurfacePointerDown,
    handleResizeSurfacePointerMove,
    handleResizeSurfacePointerEnd,
    handleResizePointerMove,
    handleResizePointerEnd,
    clearWindowFrameInteraction,
    shellHandlers,
  };
}

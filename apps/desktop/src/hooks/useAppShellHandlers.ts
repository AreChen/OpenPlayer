import type {
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
  WheelEvent as ReactWheelEvent,
} from "react";
import { DEFAULT_VOLUME_STEP } from "../app/constants";
import {
  isPointerInsidePlaybackControl,
  isPointerInsideSelector,
  isWheelInsideInteractiveSurface,
} from "../app/shortcuts";
import type { ContextMenuPosition, ShortcutAction } from "../app/types";

type UseAppShellHandlersOptions = {
  contextMenu: ContextMenuPosition | null;
  isSettingsOpen: boolean;
  isNetworkStreamDialogOpen: boolean;
  recordingShortcutAction: ShortcutAction | null;
  volumeLevel: number;
  recordUserActivity: () => void;
  openContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
  closeContextMenu: () => void;
  closeFloatingPlaybackMenus: () => void;
  handleShellPointerLeave: () => void;
  setVolume: (value: number, options?: { feedback?: boolean }) => void;
};

export function useAppShellHandlers({
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
}: UseAppShellHandlersOptions) {
  function handleShellPointerDown(event: ReactPointerEvent<HTMLElement>) {
    recordUserActivity();
    if (contextMenu && !isPointerInsideSelector(event.target, ".context-menu")) {
      closeContextMenu();
    }
    if (!isPointerInsidePlaybackControl(event.target)) {
      closeFloatingPlaybackMenus();
    }
  }

  function handleShellWheel(event: ReactWheelEvent<HTMLElement>) {
    if (
      contextMenu ||
      isSettingsOpen ||
      isNetworkStreamDialogOpen ||
      recordingShortcutAction ||
      isWheelInsideInteractiveSurface(event.target)
    ) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const direction = event.deltaY > 0 ? -1 : 1;
    setVolume(volumeLevel + direction * DEFAULT_VOLUME_STEP, { feedback: true });
  }

  return {
    onContextMenu: (event: ReactMouseEvent<HTMLElement>) => {
      recordUserActivity();
      openContextMenu(event);
    },
    onDragOver: (event: ReactMouseEvent<HTMLElement>) => event.preventDefault(),
    onDrop: (event: ReactMouseEvent<HTMLElement>) => event.preventDefault(),
    onKeyDown: recordUserActivity,
    onPointerDown: handleShellPointerDown,
    onPointerLeave: handleShellPointerLeave,
    onPointerMove: recordUserActivity,
    onWheel: handleShellWheel,
  };
}

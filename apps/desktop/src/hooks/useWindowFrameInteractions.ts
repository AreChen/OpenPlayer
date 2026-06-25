import { useEffect } from "react";
import { applyResizeCursor } from "../app/windowControls";
import { useWindowDragRegion } from "./windowFrameInteractions/drag";
import { useWindowResizeRegions } from "./windowFrameInteractions/resize";

type UseWindowFrameInteractionsOptions = {
  platformOs: string | null | undefined;
  onTogglePlayback: () => void;
  onUserActivity: () => void;
};

export function useWindowFrameInteractions({
  platformOs,
  onTogglePlayback,
  onUserActivity,
}: UseWindowFrameInteractionsOptions) {
  const {
    handleDragRegionDoubleClick,
    handleDragRegionPointerDown,
    handleDragRegionPointerMove,
    handleDragRegionPointerEnd,
    clearPendingWindowDrag,
  } = useWindowDragRegion({ onTogglePlayback });
  const {
    handleResizePointerEnter,
    handleResizePointerLeave,
    handleResizePointerDown,
    handleResizeSurfacePointerDown,
    handleResizeSurfacePointerMove,
    handleResizeSurfacePointerEnd,
    handleResizePointerMove,
    handleResizePointerEnd,
    clearResizeHoverCursor,
    clearWindowResizeInteraction,
  } = useWindowResizeRegions({ platformOs, onUserActivity });

  function clearWindowFrameInteraction() {
    clearPendingWindowDrag();
    clearWindowResizeInteraction();
  }

  useEffect(
    () => () => {
      clearWindowFrameInteraction();
      void applyResizeCursor(null);
    },
    [],
  );

  return {
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
  };
}

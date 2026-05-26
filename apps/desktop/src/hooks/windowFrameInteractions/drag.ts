import { useRef, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { WINDOW_DOUBLE_CLICK_MAX_DISTANCE_PX, WINDOW_DOUBLE_CLICK_MAX_MS } from "../../app/constants";
import { runWindowCommand, startMainWindowDrag } from "../../app/windowControls";

type UseWindowDragRegionOptions = {
  onTogglePlayback: () => void;
};

type WindowDragClick = {
  timeStamp: number;
  x: number;
  y: number;
};

function isRepeatedWindowDragClick(previous: WindowDragClick | null, event: ReactPointerEvent<HTMLDivElement>) {
  if (!previous) {
    return false;
  }

  const elapsedMs = event.timeStamp - previous.timeStamp;
  if (elapsedMs < 0 || elapsedMs > WINDOW_DOUBLE_CLICK_MAX_MS) {
    return false;
  }

  const deltaX = event.clientX - previous.x;
  const deltaY = event.clientY - previous.y;
  return deltaX * deltaX + deltaY * deltaY <= WINDOW_DOUBLE_CLICK_MAX_DISTANCE_PX * WINDOW_DOUBLE_CLICK_MAX_DISTANCE_PX;
}

export function useWindowDragRegion({ onTogglePlayback }: UseWindowDragRegionOptions) {
  const suppressNextDoubleClickRef = useRef(false);
  const doubleClickSuppressTimerRef = useRef<number | null>(null);
  const lastWindowDragClickRef = useRef<WindowDragClick | null>(null);

  function clearDoubleClickSuppress() {
    suppressNextDoubleClickRef.current = false;
    if (doubleClickSuppressTimerRef.current !== null) {
      window.clearTimeout(doubleClickSuppressTimerRef.current);
      doubleClickSuppressTimerRef.current = null;
    }
  }

  function suppressFallbackDoubleClick() {
    suppressNextDoubleClickRef.current = true;
    if (doubleClickSuppressTimerRef.current !== null) {
      window.clearTimeout(doubleClickSuppressTimerRef.current);
    }
    doubleClickSuppressTimerRef.current = window.setTimeout(() => {
      suppressNextDoubleClickRef.current = false;
      doubleClickSuppressTimerRef.current = null;
    }, WINDOW_DOUBLE_CLICK_MAX_MS);
  }

  function clearPendingWindowDrag() {}

  function handleDragRegionPointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button === 1) {
      event.preventDefault();
      lastWindowDragClickRef.current = null;
      runWindowCommand("window_toggle_fullscreen");
      return;
    }

    if (event.button === 0) {
      if (isRepeatedWindowDragClick(lastWindowDragClickRef.current, event)) {
        event.preventDefault();
        event.stopPropagation();
        lastWindowDragClickRef.current = null;
        suppressFallbackDoubleClick();
        onTogglePlayback();
        return;
      }
      lastWindowDragClickRef.current = {
        timeStamp: event.timeStamp,
        x: event.clientX,
        y: event.clientY,
      };
      event.preventDefault();
      startMainWindowDrag();
    }
  }

  function handleDragRegionPointerMove(_event: ReactPointerEvent<HTMLDivElement>) {}

  function handleDragRegionPointerEnd(_event: ReactPointerEvent<HTMLDivElement>) {}

  function handleDragRegionDoubleClick(event: ReactMouseEvent<HTMLDivElement>) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    if (suppressNextDoubleClickRef.current) {
      clearDoubleClickSuppress();
      return;
    }
    onTogglePlayback();
  }

  return {
    handleDragRegionDoubleClick,
    handleDragRegionPointerDown,
    handleDragRegionPointerMove,
    handleDragRegionPointerEnd,
    clearPendingWindowDrag,
  };
}

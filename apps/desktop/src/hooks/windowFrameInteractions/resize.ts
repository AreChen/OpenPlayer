import { useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import type { ManualResizeDrag, ResizeDirection, ResizeFeedback } from "../../app/types";
import {
  applyManualMainWindowResize,
  applyResizeCursor,
  startNativeMainWindowResize,
} from "../../app/windowControls";

type UseWindowResizeRegionsOptions = {
  platformOs: string | null | undefined;
  onUserActivity: () => void;
};

function isMacosResizeRuntime(platformOs: string | null | undefined) {
  return platformOs === "macos" || (!platformOs && typeof navigator !== "undefined" && /Mac/.test(navigator.platform));
}

export function useWindowResizeRegions({ platformOs, onUserActivity }: UseWindowResizeRegionsOptions) {
  const [resizeFeedback, setResizeFeedback] = useState<ResizeFeedback | null>(null);
  const manualResizeDragRef = useRef<ManualResizeDrag | null>(null);
  const resizeCursorDirectionRef = useRef<ResizeDirection | null>(null);

  function clearManualResizeDrag() {
    const pendingResize = manualResizeDragRef.current;
    if (pendingResize?.animationFrameId != null) {
      window.cancelAnimationFrame(pendingResize.animationFrameId);
    }
    manualResizeDragRef.current = null;
  }

  function setNativeResizeCursor(direction: ResizeDirection | null) {
    if (resizeCursorDirectionRef.current === direction) {
      return;
    }

    resizeCursorDirectionRef.current = direction;
    void applyResizeCursor(direction);
  }

  function setResizeBoundaryFeedback(direction: ResizeDirection | null, active = false) {
    setResizeFeedback((feedback) => {
      if (!direction) {
        return feedback === null ? feedback : null;
      }

      if (feedback?.direction === direction && feedback.active === active) {
        return feedback;
      }

      return { direction, active };
    });
  }

  function completeManualResizeIfIdle(pendingResize: ManualResizeDrag) {
    if (
      pendingResize.finishing &&
      !pendingResize.resizeCommandInFlight &&
      pendingResize.animationFrameId === null &&
      Math.abs(pendingResize.pendingDeltaX) < 0.5 &&
      Math.abs(pendingResize.pendingDeltaY) < 0.5 &&
      manualResizeDragRef.current === pendingResize
    ) {
      manualResizeDragRef.current = null;
    }
  }

  function requestManualResizeFlush() {
    const pendingResize = manualResizeDragRef.current;
    if (!pendingResize || pendingResize.animationFrameId !== null || pendingResize.resizeCommandInFlight) {
      return;
    }

    pendingResize.animationFrameId = window.requestAnimationFrame(() => {
      const activeResize = manualResizeDragRef.current;
      if (!activeResize) {
        return;
      }

      activeResize.animationFrameId = null;
      flushManualResizeDelta();
    });
  }

  function flushManualResizeDelta() {
    const pendingResize = manualResizeDragRef.current;
    if (!pendingResize || pendingResize.resizeCommandInFlight) {
      return;
    }

    const deltaX = pendingResize.pendingDeltaX;
    const deltaY = pendingResize.pendingDeltaY;
    if (Math.abs(deltaX) < 0.5 && Math.abs(deltaY) < 0.5) {
      completeManualResizeIfIdle(pendingResize);
      return;
    }

    pendingResize.pendingDeltaX = 0;
    pendingResize.pendingDeltaY = 0;
    pendingResize.resizeCommandInFlight = true;
    applyManualMainWindowResize(pendingResize.direction, deltaX, deltaY).finally(() => {
      if (manualResizeDragRef.current !== pendingResize) {
        return;
      }

      pendingResize.resizeCommandInFlight = false;
      if (Math.abs(pendingResize.pendingDeltaX) >= 0.5 || Math.abs(pendingResize.pendingDeltaY) >= 0.5) {
        requestManualResizeFlush();
        return;
      }

      completeManualResizeIfIdle(pendingResize);
    });
  }

  function startMainWindowResize(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    setNativeResizeCursor(direction);
    setResizeBoundaryFeedback(direction, true);
    if (isMacosResizeRuntime(platformOs)) {
      event.currentTarget.setPointerCapture(event.pointerId);
      manualResizeDragRef.current = {
        pointerId: event.pointerId,
        direction,
        lastX: event.clientX,
        lastY: event.clientY,
        pendingDeltaX: 0,
        pendingDeltaY: 0,
        animationFrameId: null,
        resizeCommandInFlight: false,
        finishing: false,
      };
      return;
    }

    startNativeMainWindowResize(direction);
  }

  function handleResizePointerEnter(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    event.stopPropagation();
    setNativeResizeCursor(direction);
    setResizeBoundaryFeedback(direction);
  }

  function handleResizePointerLeave(event: ReactPointerEvent<HTMLDivElement>) {
    if (manualResizeDragRef.current?.pointerId === event.pointerId) {
      return;
    }

    event.stopPropagation();
    setNativeResizeCursor(null);
    setResizeBoundaryFeedback(null);
  }

  function handleResizePointerDown(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    onUserActivity();
    startMainWindowResize(event, direction);
  }

  function handleResizePointerMove(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    const pendingResize = manualResizeDragRef.current;
    if (!pendingResize || pendingResize.pointerId !== event.pointerId) {
      event.stopPropagation();
      setNativeResizeCursor(direction);
      setResizeBoundaryFeedback(direction);
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    onUserActivity();
    const scale = window.devicePixelRatio || 1;
    const deltaX = (event.clientX - pendingResize.lastX) * scale;
    const deltaY = (event.clientY - pendingResize.lastY) * scale;
    if (Math.abs(deltaX) < 0.5 && Math.abs(deltaY) < 0.5) {
      return;
    }
    pendingResize.lastX = event.clientX;
    pendingResize.lastY = event.clientY;
    pendingResize.pendingDeltaX += deltaX;
    pendingResize.pendingDeltaY += deltaY;
    requestManualResizeFlush();
  }

  function handleResizePointerEnd(event: ReactPointerEvent<HTMLDivElement>) {
    const pendingResize = manualResizeDragRef.current;
    if (pendingResize?.pointerId !== event.pointerId) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    pendingResize.finishing = true;
    requestManualResizeFlush();
    completeManualResizeIfIdle(pendingResize);
    setNativeResizeCursor(null);
    setResizeBoundaryFeedback(null);
  }

  function clearResizeHoverFeedback() {
    if (!manualResizeDragRef.current) {
      setNativeResizeCursor(null);
      setResizeBoundaryFeedback(null);
    }
  }

  function clearWindowResizeInteraction() {
    clearManualResizeDrag();
    setNativeResizeCursor(null);
    setResizeBoundaryFeedback(null);
  }

  return {
    resizeFeedback,
    handleResizePointerEnter,
    handleResizePointerLeave,
    handleResizePointerDown,
    handleResizePointerMove,
    handleResizePointerEnd,
    clearResizeHoverFeedback,
    clearWindowResizeInteraction,
  };
}

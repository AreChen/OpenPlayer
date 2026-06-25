import { useEffect, useRef, useState } from "react";
import { AUTO_HIDE_CONTROLS_MS } from "../app/constants";
import type { ShortcutAction } from "../app/types";

type UseChromeAutoHideOptions = {
  mediaId: string | null | undefined;
  isChromePinned: boolean;
  quietKeyboardControls: boolean;
  onPointerExit?: () => void;
};

export function useChromeAutoHide({ mediaId, isChromePinned, quietKeyboardControls, onPointerExit }: UseChromeAutoHideOptions) {
  const [isChromeVisible, setIsChromeVisible] = useState(true);
  const chromeHideTimerRef = useRef<number | null>(null);

  function clearChromeHideTimer() {
    if (chromeHideTimerRef.current !== null) {
      window.clearTimeout(chromeHideTimerRef.current);
      chromeHideTimerRef.current = null;
    }
  }

  function scheduleChromeHide() {
    clearChromeHideTimer();
    if (isChromePinned) {
      return;
    }

    chromeHideTimerRef.current = window.setTimeout(() => {
      setIsChromeVisible(false);
      chromeHideTimerRef.current = null;
    }, AUTO_HIDE_CONTROLS_MS);
  }

  function recordUserActivity() {
    setIsChromeVisible(true);
    scheduleChromeHide();
  }

  function recordShortcutActivity(action: ShortcutAction) {
    if (quietKeyboardControls && ["seekBackward", "seekForward", "volumeDown", "volumeUp"].includes(action)) {
      return;
    }

    recordUserActivity();
  }

  function hideChromeForPointerExit() {
    clearChromeHideTimer();
    onPointerExit?.();
    if (!isChromePinned && mediaId) {
      setIsChromeVisible(false);
    }
  }

  function handleShellPointerLeave() {
    hideChromeForPointerExit();
  }

  useEffect(() => {
    setIsChromeVisible(true);
    scheduleChromeHide();
    return clearChromeHideTimer;
  }, [mediaId, isChromePinned]);

  useEffect(() => {
    const handleWindowPointerExit = (event: MouseEvent | PointerEvent) => {
      const relatedTarget = event.relatedTarget;
      if (relatedTarget instanceof Node && document.documentElement.contains(relatedTarget)) {
        return;
      }
      hideChromeForPointerExit();
    };

    const handleWindowBlur = () => {
      hideChromeForPointerExit();
    };

    window.addEventListener("mouseout", handleWindowPointerExit);
    window.addEventListener("pointerout", handleWindowPointerExit);
    window.addEventListener("blur", handleWindowBlur);
    document.documentElement.addEventListener("mouseleave", handleWindowBlur);
    return () => {
      window.removeEventListener("mouseout", handleWindowPointerExit);
      window.removeEventListener("pointerout", handleWindowPointerExit);
      window.removeEventListener("blur", handleWindowBlur);
      document.documentElement.removeEventListener("mouseleave", handleWindowBlur);
    };
  }, [mediaId, isChromePinned]);

  return {
    isChromeVisible,
    recordUserActivity,
    recordShortcutActivity,
    handleShellPointerLeave,
  };
}

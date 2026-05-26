import { useEffect, useRef, useState } from "react";
import { PLAYBACK_ERROR_FEEDBACK_MS, VOLUME_FEEDBACK_MS } from "../app/constants";
import type { AlwaysOnTopFeedback, CaptureFeedback, VolumeFeedback } from "../app/types";

export function usePlayerFeedback() {
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [volumeFeedback, setVolumeFeedback] = useState<VolumeFeedback | null>(null);
  const [alwaysOnTopFeedback, setAlwaysOnTopFeedback] = useState<AlwaysOnTopFeedback | null>(null);
  const [captureFeedback, setCaptureFeedback] = useState<CaptureFeedback | null>(null);
  const playbackErrorTimerRef = useRef<number | null>(null);
  const volumeFeedbackTimerRef = useRef<number | null>(null);
  const alwaysOnTopFeedbackTimerRef = useRef<number | null>(null);
  const captureFeedbackTimerRef = useRef<number | null>(null);

  function showVolumeFeedback(level: number) {
    const nextLevel = Math.min(1, Math.max(0, level));
    setVolumeFeedback({ level: nextLevel });
    if (volumeFeedbackTimerRef.current !== null) {
      window.clearTimeout(volumeFeedbackTimerRef.current);
    }
    volumeFeedbackTimerRef.current = window.setTimeout(() => {
      setVolumeFeedback(null);
      volumeFeedbackTimerRef.current = null;
    }, VOLUME_FEEDBACK_MS);
  }

  function showAlwaysOnTopFeedback(enabled: boolean) {
    setAlwaysOnTopFeedback({ enabled });
    if (alwaysOnTopFeedbackTimerRef.current !== null) {
      window.clearTimeout(alwaysOnTopFeedbackTimerRef.current);
    }
    alwaysOnTopFeedbackTimerRef.current = window.setTimeout(() => {
      setAlwaysOnTopFeedback(null);
      alwaysOnTopFeedbackTimerRef.current = null;
    }, VOLUME_FEEDBACK_MS);
  }

  function showCaptureFeedback(icon: CaptureFeedback["icon"], message: string) {
    setCaptureFeedback({ icon, message });
    if (captureFeedbackTimerRef.current !== null) {
      window.clearTimeout(captureFeedbackTimerRef.current);
    }
    captureFeedbackTimerRef.current = window.setTimeout(() => {
      setCaptureFeedback(null);
      captureFeedbackTimerRef.current = null;
    }, VOLUME_FEEDBACK_MS);
  }

  useEffect(() => {
    if (playbackErrorTimerRef.current !== null) {
      window.clearTimeout(playbackErrorTimerRef.current);
      playbackErrorTimerRef.current = null;
    }
    if (!playbackError) {
      return;
    }

    playbackErrorTimerRef.current = window.setTimeout(() => {
      setPlaybackError(null);
      playbackErrorTimerRef.current = null;
    }, PLAYBACK_ERROR_FEEDBACK_MS);
  }, [playbackError]);

  useEffect(() => {
    return () => {
      if (playbackErrorTimerRef.current !== null) {
        window.clearTimeout(playbackErrorTimerRef.current);
      }
      if (volumeFeedbackTimerRef.current !== null) {
        window.clearTimeout(volumeFeedbackTimerRef.current);
      }
      if (alwaysOnTopFeedbackTimerRef.current !== null) {
        window.clearTimeout(alwaysOnTopFeedbackTimerRef.current);
      }
      if (captureFeedbackTimerRef.current !== null) {
        window.clearTimeout(captureFeedbackTimerRef.current);
      }
    };
  }, []);

  return {
    playbackError,
    setPlaybackError,
    volumeFeedback,
    alwaysOnTopFeedback,
    captureFeedback,
    showVolumeFeedback,
    showAlwaysOnTopFeedback,
    showCaptureFeedback,
  };
}

import { platformUnsupportedPlaybackMessage } from "../app/playback";
import type { AppStrings } from "../i18n";
import type { PlatformSupport } from "../app/types";

type UsePlaybackErrorReporterOptions = {
  platformSupport: PlatformSupport | null;
  t: AppStrings;
  setPlaybackError: (error: string | null) => void;
};

export function usePlaybackErrorReporter({ platformSupport, t, setPlaybackError }: UsePlaybackErrorReporterOptions) {
  function reportPlaybackError(error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("mpv has no loaded media")) {
      return;
    }

    if (
      message.includes("mpv embed playback currently supports Windows HWND hosts only") ||
      message.includes("video host support is not implemented yet")
    ) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return;
    }

    setPlaybackError(message);
  }

  return {
    reportPlaybackError,
  };
}

import { useEffect, useRef, useState } from "react";
import { getCurrentWebview, type DragDropEvent } from "@tauri-apps/api/webview";

type UseMediaDropTargetOptions = {
  onDropPaths: (paths: string[]) => void;
};

export function useMediaDropTarget({ onDropPaths }: UseMediaDropTargetOptions) {
  const [isDropActive, setIsDropActive] = useState(false);
  const droppedPathsHandlerRef = useRef<(paths: string[]) => void>(() => undefined);

  droppedPathsHandlerRef.current = onDropPaths;

  useEffect(() => {
    let disposed = false;
    let unlistenDrop: (() => void) | null = null;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload: DragDropEvent = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          setIsDropActive(true);
          return;
        }

        setIsDropActive(false);
        if (payload.type === "drop" && payload.paths.length > 0) {
          droppedPathsHandlerRef.current(payload.paths);
        }
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenDrop = unlisten;
        }
      })
      .catch((error: unknown) => {
        console.warn("File drop listener failed", error);
      });

    return () => {
      disposed = true;
      unlistenDrop?.();
    };
  }, []);

  return { isDropActive };
}

import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LoopMode, MediaItem, MpvSnapshot } from "../app/types";

type UseLoopModeSyncOptions = {
  media: MediaItem | null;
  loadedMediaPath: string | null;
  loopMode: LoopMode;
  applySnapshot: (snapshot: MpvSnapshot) => void;
  onError: (error: unknown) => void;
};

export function useLoopModeSync({
  media,
  loadedMediaPath,
  loopMode,
  applySnapshot,
  onError,
}: UseLoopModeSyncOptions) {
  const applySnapshotRef = useRef(applySnapshot);
  const onErrorRef = useRef(onError);

  useEffect(() => {
    applySnapshotRef.current = applySnapshot;
    onErrorRef.current = onError;
  }, [applySnapshot, onError]);

  useEffect(() => {
    if (!media || loadedMediaPath !== media.path) {
      return;
    }

    invoke<MpvSnapshot>("mpv_embed_set_loop_file", { enabled: loopMode === "one" })
      .then((snapshot) => {
        if (loopMode === "one") {
          applySnapshotRef.current(snapshot);
        }
      })
      .catch(onErrorRef.current);
  }, [loadedMediaPath, loopMode, media?.id, media?.path]);
}

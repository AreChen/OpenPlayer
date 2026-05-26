import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { MpvSnapshot } from "../app/types";

type UseMpvSnapshotPollingOptions = {
  mediaId: string | null | undefined;
  applySnapshot: (snapshot: MpvSnapshot) => void;
};

export function useMpvSnapshotPolling({ mediaId, applySnapshot }: UseMpvSnapshotPollingOptions) {
  const snapshotRequestIdRef = useRef(0);
  const applySnapshotRef = useRef(applySnapshot);
  applySnapshotRef.current = applySnapshot;

  function invalidatePendingSnapshots() {
    snapshotRequestIdRef.current += 1;
  }

  useEffect(() => {
    if (!mediaId) {
      return;
    }

    const timer = window.setInterval(() => {
      const requestId = ++snapshotRequestIdRef.current;
      invoke<MpvSnapshot | null>("mpv_embed_snapshot")
        .then((snapshot) => {
          if (snapshot && requestId === snapshotRequestIdRef.current) {
            applySnapshotRef.current(snapshot);
          }
        })
        .catch(() => undefined);
    }, 500);

    return () => {
      window.clearInterval(timer);
      invalidatePendingSnapshots();
    };
  }, [mediaId]);

  return {
    invalidatePendingSnapshots,
  };
}

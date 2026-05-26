import { invoke } from "@tauri-apps/api/core";
import type { MpvSnapshot, PendingSeek } from "../types";
import type {
  AnchorDisplayClock,
  RefValue,
  ReportError,
  SetValue,
} from "./shared";

export function seekTargetForDuration(value: number, duration: number) {
  if (!Number.isFinite(value)) {
    return 0;
  }

  const upperBound = duration > 0 ? duration : value;
  return Math.min(upperBound, Math.max(0, value));
}

type SeekPreviewOptions = {
  value: number;
  duration: number;
  pendingSeekRef: RefValue<PendingSeek | null>;
  setCurrentTime: SetValue<number>;
  anchorDisplayClock: AnchorDisplayClock;
};

export function previewMpvSeek({
  value,
  duration,
  pendingSeekRef,
  setCurrentTime,
  anchorDisplayClock,
}: SeekPreviewOptions) {
  const target = seekTargetForDuration(value, duration);
  pendingSeekRef.current = { target, startedAt: performance.now() };
  setCurrentTime(target);
  anchorDisplayClock(target, false);
}

type CommitSeekOptions = SeekPreviewOptions & {
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  onError: ReportError;
};

export function commitMpvSeek({
  value,
  duration,
  pendingSeekRef,
  setCurrentTime,
  anchorDisplayClock,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  onError,
}: CommitSeekOptions) {
  const target = seekTargetForDuration(value, duration);
  pendingSeekRef.current = { target, startedAt: performance.now() };
  setCurrentTime(target);
  anchorDisplayClock(target, false);
  invalidatePendingSnapshots();
  invoke<MpvSnapshot>("mpv_embed_seek", { position: target })
    .then(applyCommandSnapshot)
    .catch((error: unknown) => {
      pendingSeekRef.current = null;
      onError(error);
    });
}

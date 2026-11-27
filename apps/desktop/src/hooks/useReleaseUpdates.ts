import { useEffect, useEffectEvent, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { canCheckForUpdates } from "../app/updates";
import type { UpdateOffer, UpdateState } from "../app/types";

type CheckResult =
  | { status: "current" | "unsupported" }
  | { status: "available" | "installed"; update: UpdateOffer };

const FIRST_UPDATE_CHECK_DELAY_MS = 15_000;
const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1_000;

export function useReleaseUpdates() {
  const [updateState, setUpdateState] = useState<UpdateState>({ status: "idle" });
  const pending = useRef(false);

  async function checkForUpdates() {
    if (pending.current || !canCheckForUpdates(updateState)) return;

    pending.current = true;
    setUpdateState({ status: "checking" });
    try {
      setUpdateState(await invoke<CheckResult>("app_update_check"));
    } catch (error) {
      setUpdateState({ status: "checkFailed", message: String(error) });
    } finally {
      pending.current = false;
    }
  }

  async function runUpdateAction() {
    if (pending.current || !("update" in updateState)) return;

    const { status, update } = updateState;
    pending.current = true;
    try {
      switch (status) {
        case "available":
        case "downloadFailed":
        case "installFailed":
          setUpdateState({ status: "downloading", update });
          await invoke("app_update_download", { version: update.version });
          setUpdateState({ status: "ready", update });
          break;
        case "ready":
        case "installerOpened": {
          setUpdateState({ status: "installing", update });
          const result = await invoke<{ status: "installed" | "installerOpened" }>(
            "app_update_install", { version: update.version },
          );
          setUpdateState({ status: result.status, update });
          break;
        }
        case "installed":
        case "restartFailed":
          await invoke("app_update_restart");
          break;
      }
    } catch (error) {
      const failure = status === "ready" || status === "installerOpened" ? "installFailed"
        : status === "installed" || status === "restartFailed" ? "restartFailed" : "downloadFailed";
      setUpdateState({ status: failure, update, message: String(error) });
    } finally {
      pending.current = false;
    }
  }

  const checkInBackground = useEffectEvent(() => {
    void checkForUpdates();
  });

  useEffect(() => {
    const initial = window.setTimeout(checkInBackground, FIRST_UPDATE_CHECK_DELAY_MS);
    const daily = window.setInterval(checkInBackground, UPDATE_CHECK_INTERVAL_MS);
    return () => {
      window.clearTimeout(initial);
      window.clearInterval(daily);
    };
  }, []);

  return { updateState, checkForUpdates, runUpdateAction };
}

import { invoke } from "@tauri-apps/api/core";
import type { ResizeDirection, WindowCommand } from "./types";

export function focusOverlayWindow() {
  invoke("window_focus_overlay").catch((error: unknown) => {
    console.warn("Overlay focus restore failed", error);
  });
}

export function runWindowCommand(command: WindowCommand) {
  invoke(command)
    .then(() => {
      if (command !== "window_close" && command !== "window_minimize") {
        focusOverlayWindow();
      }
    })
    .catch((error: unknown) => {
      console.error(`Window command failed: ${command}`, error);
    });
}

export function startMainWindowDrag() {
  invoke("window_start_drag").catch((error: unknown) => {
    console.error("Window drag failed", error);
  });
}

export function startNativeMainWindowResize(direction: ResizeDirection) {
  invoke("window_start_resize", { direction }).catch((error: unknown) => {
    console.error(`Window resize failed: ${direction}`, error);
  });
}

export function applyManualMainWindowResize(direction: ResizeDirection, deltaX: number, deltaY: number) {
  return invoke("window_apply_resize_delta", { direction, deltaX, deltaY }).catch((error: unknown) => {
    console.error(`Window resize failed: ${direction}`, error);
  });
}

export function applyResizeCursor(direction: ResizeDirection | null) {
  return invoke("window_set_resize_cursor", { direction }).catch((error: unknown) => {
    console.warn("Resize cursor update failed", error);
  });
}

export function resizeDirectionClassName(direction: ResizeDirection) {
  return direction.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

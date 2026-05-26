import { useState, type MouseEvent as ReactMouseEvent } from "react";
import { CONTEXT_MENU_HEIGHT, CONTEXT_MENU_WIDTH } from "../app/constants";
import type { ContextMenuPosition } from "../app/types";

export function useContextMenuState() {
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null);

  function openContextMenu(event: ReactMouseEvent<HTMLElement>) {
    event.preventDefault();
    const x = Math.min(Math.max(8, event.clientX), Math.max(8, window.innerWidth - CONTEXT_MENU_WIDTH - 8));
    const y = Math.min(Math.max(8, event.clientY), Math.max(8, window.innerHeight - CONTEXT_MENU_HEIGHT - 8));
    setContextMenu({ x, y });
  }

  function closeContextMenu() {
    setContextMenu(null);
  }

  return {
    contextMenu,
    setContextMenu,
    openContextMenu,
    closeContextMenu,
  };
}

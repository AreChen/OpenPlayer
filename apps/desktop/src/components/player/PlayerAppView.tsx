import type { ComponentProps, PointerEvent } from "react";
import { resizeRegions } from "../../app/constants";
import type { ResizeDirection, ThemeStyleProperties } from "../../app/types";
import { ContextMenu } from "../ContextMenu";
import { NetworkStreamDialog } from "../NetworkStreamDialog";
import { PluginViewShell } from "../plugins/PluginViewShell";
import { SettingsDialog } from "../settings/SettingsDialog";
import { LoopPanel, SpeedPanel, TracksPanel } from "./MediaPanels";
import { PlaylistDrawer } from "./PlaylistDrawer";
import { StageOverlays } from "./StageOverlays";
import { TransportControls } from "./TransportControls";

type ShellHandlers = Pick<
  ComponentProps<"main">,
  | "onContextMenu"
  | "onDragOver"
  | "onDrop"
  | "onKeyDown"
  | "onPointerDown"
  | "onPointerLeave"
  | "onPointerMove"
  | "onWheel"
>;

type DragRegionHandlers = Pick<
  ComponentProps<"div">,
  | "onAuxClick"
  | "onDragStart"
  | "onDoubleClick"
  | "onPointerDown"
  | "onPointerMove"
  | "onPointerUp"
  | "onPointerCancel"
>;

type ResizeRegionHandlers = {
  onPointerEnter: (event: PointerEvent<HTMLDivElement>, direction: ResizeDirection) => void;
  onPointerLeave: ComponentProps<"div">["onPointerLeave"];
  onPointerDown: (event: PointerEvent<HTMLDivElement>, direction: ResizeDirection) => void;
  onPointerMove: (event: PointerEvent<HTMLDivElement>, direction: ResizeDirection) => void;
  onPointerUp: ComponentProps<"div">["onPointerUp"];
  onPointerCancel: ComponentProps<"div">["onPointerCancel"];
};

type PlayerAppViewProps = {
  appearanceStyle: ThemeStyleProperties | undefined;
  mediaLoaded: boolean;
  isChromeHidden: boolean;
  shellHandlers: ShellHandlers;
  stageOverlaysProps: ComponentProps<typeof StageOverlays>;
  dragRegionHandlers: DragRegionHandlers;
  resizeRegionHandlers: ResizeRegionHandlers;
  transportControlsProps: ComponentProps<typeof TransportControls>;
  speedPanelProps: ComponentProps<typeof SpeedPanel> | null;
  loopPanelProps: ComponentProps<typeof LoopPanel> | null;
  tracksPanelProps: ComponentProps<typeof TracksPanel> | null;
  playlistDrawerProps: ComponentProps<typeof PlaylistDrawer> | null;
  contextMenuProps: ComponentProps<typeof ContextMenu> | null;
  pluginViewShellProps: ComponentProps<typeof PluginViewShell> | null;
  networkStreamDialogProps: ComponentProps<typeof NetworkStreamDialog> | null;
  settingsDialogProps: ComponentProps<typeof SettingsDialog> | null;
};

export function PlayerAppView({
  appearanceStyle,
  mediaLoaded,
  isChromeHidden,
  shellHandlers,
  stageOverlaysProps,
  dragRegionHandlers,
  resizeRegionHandlers,
  transportControlsProps,
  speedPanelProps,
  loopPanelProps,
  tracksPanelProps,
  playlistDrawerProps,
  contextMenuProps,
  pluginViewShellProps,
  networkStreamDialogProps,
  settingsDialogProps,
}: PlayerAppViewProps) {
  return (
    <main className="app-shell" style={appearanceStyle} {...shellHandlers}>
      <section
        className={`window-shell ${mediaLoaded ? "window-shell--loaded" : ""}`}
        aria-label="OpenPlayer"
      >
        <section
          className={`stage ${mediaLoaded ? "stage--loaded" : ""} ${isChromeHidden ? "stage--chrome-hidden" : ""}`}
          aria-label="Player surface"
        >
          <StageOverlays {...stageOverlaysProps} />

          <div className="drag-region" aria-hidden="true" draggable={false} {...dragRegionHandlers} />

          {resizeRegions.map((region) => (
            <div
              key={region.direction}
              aria-hidden="true"
              className={`resize-region ${region.className}`}
              onPointerEnter={(event) => resizeRegionHandlers.onPointerEnter(event, region.direction)}
              onPointerLeave={resizeRegionHandlers.onPointerLeave}
              onPointerDown={(event) => resizeRegionHandlers.onPointerDown(event, region.direction)}
              onPointerMove={(event) => resizeRegionHandlers.onPointerMove(event, region.direction)}
              onPointerUp={resizeRegionHandlers.onPointerUp}
              onPointerCancel={resizeRegionHandlers.onPointerCancel}
            />
          ))}

          <TransportControls {...transportControlsProps} />

          {speedPanelProps && <SpeedPanel {...speedPanelProps} />}
          {loopPanelProps && <LoopPanel {...loopPanelProps} />}
          {tracksPanelProps && <TracksPanel {...tracksPanelProps} />}
          {playlistDrawerProps && <PlaylistDrawer {...playlistDrawerProps} />}
          {contextMenuProps && <ContextMenu {...contextMenuProps} />}
          {pluginViewShellProps && <PluginViewShell {...pluginViewShellProps} />}
          {networkStreamDialogProps && <NetworkStreamDialog {...networkStreamDialogProps} />}
          {settingsDialogProps && <SettingsDialog {...settingsDialogProps} />}
        </section>
      </section>
    </main>
  );
}

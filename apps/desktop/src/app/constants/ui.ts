import type { ResizeDirection } from "../types";

export const playbackSpeedOptions = [0.5, 0.75, 1, 1.25, 1.5, 2];
export const accentSwatches = ["#caa05d", "#78d5b3", "#93b4ff", "#d78372", "#b48cf2", "#e4b95f"];
export const audioVisualizerBarLevels = [
  0.34, 0.56, 0.42, 0.72, 0.5, 0.82, 0.46, 0.66, 0.38, 0.92, 0.58, 0.76, 0.44, 0.68, 0.52,
  0.86, 0.48, 0.62, 0.36, 0.74, 0.54, 0.88, 0.4, 0.7,
];

export const CONTEXT_MENU_WIDTH = 236;
export const CONTEXT_MENU_HEIGHT = 420;

export const resizeRegions: Array<{ className: string; direction: ResizeDirection }> = [
  { className: "resize-region--north", direction: "North" },
  { className: "resize-region--south", direction: "South" },
  { className: "resize-region--east", direction: "East" },
  { className: "resize-region--west", direction: "West" },
  { className: "resize-region--north-east", direction: "NorthEast" },
  { className: "resize-region--north-west", direction: "NorthWest" },
  { className: "resize-region--south-east", direction: "SouthEast" },
  { className: "resize-region--south-west", direction: "SouthWest" },
];

export const surface = new URLSearchParams(window.location.search).get("surface");
export const openPlayerLogoUrl = new URL("../../assets/openplayer-logo.png", import.meta.url).href;

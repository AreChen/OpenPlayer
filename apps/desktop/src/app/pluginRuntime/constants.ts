export const supportedPluginLoadOptionKeys = new Set(["demuxer", "demuxer-lavf-format"]);
export const PLUGIN_SDK_VERSION = "1.6.1";
export const OPENPLAYER_HOST_VERSION = "1.6.1";
export const OPENPLAYER_API_COMPATIBILITY = Object.freeze({
  sdkVersion: PLUGIN_SDK_VERSION,
  hostVersion: OPENPLAYER_HOST_VERSION,
  minHostVersion: "1.6.0",
  compatibility: "1.x-additive",
});
export const OPENPLAYER_HOST_CAPABILITIES = Object.freeze([
  "app.ready",
  "api.compatibility",
  "events.subscribe",
  "plugin.commands",
  "plugin.logs",
  "plugin.settings",
  "plugin.artifacts",
  "plugin.storage",
  "plugin.tasks",
  "plugin.views",
  "player.playback",
  "player.tracks",
  "player.snapshot",
  "playlist.read",
  "playlist.write",
  "media.openStream",
  "media.segments",
  "media.segmentExport",
  "audio.extractClip",
  "capture.frame",
  "mpv.loadOptions",
  "mpv.capture",
  "mpv.wall",
  "mpv.core",
  "mpv.filters",
  "mpv.filters.audio",
  "mpv.abLoop",
  "mpv.osd",
  "mpv.scriptMessage",
  "network.request",
  "network.json",
  "filesystem.pick",
  "filesystem.reveal",
  "subtitle.external",
  "subtitle.read",
  "subtitle.style",
  "subtitle.documents",
  "subtitle.generated",
  "subtitle.cues",
  "ui.toast",
  "ui.panels",
  "ui.permissionRisk",
  "ui.settings",
]);
export const PLUGIN_HOOK_TIMEOUT_MS = 750;
export const PLUGIN_COMMAND_HOOK_TIMEOUT_MS = 10_000;
export const MAX_PLUGIN_NETWORK_TIMEOUT_MS = 30_000;
export const MAX_PLUGIN_WALL_TILES = 16;
export const supportedPluginNetworkMethods = new Set(["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]);
export const PLUGIN_VIEW_BRIDGE_ID = "openplayer-plugin-view";

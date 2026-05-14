import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const mainSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const tauriLibSource = await readFile(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
const capability = JSON.parse(await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));

function extractGenerateHandler(source) {
  const match = source.match(/tauri::generate_handler!\[([\s\S]*?)\]\)/);
  assert.ok(match, "Tauri invoke handler must be registered with generate_handler");
  return match[1];
}

const tauriGenerateHandler = extractGenerateHandler(tauriLibSource);

const frontendPlaybackCommands = [
  "playback_open_preview_source",
  "playback_play",
  "playback_pause",
  "playback_stop",
  "playback_seek",
  "playback_set_volume",
];

const storageCommands = [
  "storage_recent_media_list",
  "storage_recent_media_record",
  "storage_progress_get",
  "storage_progress_save",
  "storage_progress_clear",
  "storage_setting_get",
  "storage_setting_set",
];

const [mainWindow] = config.app.windows;
const assetProtocol = config.app.security.assetProtocol;
const dialogPluginIndex = tauriLibSource.indexOf(".plugin(tauri_plugin_dialog::init())");
const playbackStateIndex = tauriLibSource.indexOf(".manage(DesktopPlaybackState::default())");

assert.equal(mainWindow.url, "index.html", "packaged exe must load the bundled app entry");
assert.equal(mainWindow.decorations, false, "window must disable native decorations for custom titlebar");
assert.equal(mainWindow.transparent, false, "window must not use transparency that exposes an outer border");
assert.equal(mainWindow.shadow, true, "window should keep native shadow when available");
assert.match(config.build.devUrl, /23142$/, "Tauri dev URL must use the non-reserved Windows port");
assert.match(packageJson.scripts.dev, /23142$/, "Vite dev script must use the non-reserved Windows port");
assert.match(packageJson.scripts.preview, /23142$/, "Vite preview script must use the non-reserved Windows port");
assert.doesNotMatch(appSource, /titlebar-brand/, "player should not show a top brand/title block");
assert.doesNotMatch(appSource, /titlebar-center/, "player should not show a theme/status title pill");
assert.doesNotMatch(appSource, /side-rail/, "playlist and metadata must not be a fixed right sidebar");
assert.doesNotMatch(appSource, /status-line/, "player should not show debug/status copy over the video surface");
assert.match(appSource, /className={`stage/, "the playback stage must be the main window surface");
assert.doesNotMatch(appSource, /data-tauri-drag-region/, "native drag regions must not wrap player controls because they steal clicks");
assert.match(appSource, /getCurrentWindow/, "player must use Tauri's imperative window API for reliable drag");
assert.match(appSource, /startDragging/, "player surface must start native dragging from pointer events");
assert.match(appSource, /drag-surface/, "player must use an isolated drag layer behind controls");
assert.match(appSource, /beginWindowDragIntent/, "player must record drag intent before starting native drag");
assert.match(appSource, /continueWindowDragIntent/, "player must start native dragging only after pointer movement");
assert.match(appSource, /onDoubleClick=\{toggleFullscreen\}/, "player surface double click must toggle fullscreen");
assert.match(appSource, /setFullscreen/, "player must call Tauri fullscreen API on double click");
assert.match(appSource, /window_minimize/, "custom titlebar must wire minimize command");
assert.match(appSource, /window_toggle_maximize/, "custom titlebar must wire maximize command");
assert.match(appSource, /window_close/, "custom titlebar must wire close command");
assert.match(appSource, /playlist-drawer/, "playlist must be a collapsible drawer");
assert.match(appSource, /togglePlaylist/, "control bar must wire a playlist toggle");
assert.match(appSource, /<video/, "player shell must include an actual media element");
assert.ok(packageJson.dependencies["@tauri-apps/plugin-dialog"], "desktop package must depend on Tauri dialog plugin");
assert.equal(assetProtocol?.enable, true, "Tauri asset protocol must be enabled for local preview URLs");
assert.ok(assetProtocol?.scope?.includes("**"), "asset protocol scope must allow user-selected local media paths");
assert.ok(capability.permissions.includes("dialog:allow-open"), "capability must allow native file-open dialogs");
assert.notEqual(dialogPluginIndex, -1, "desktop app must register the dialog plugin");
assert.notEqual(playbackStateIndex, -1, "desktop app must manage playback state");
assert.ok(dialogPluginIndex < playbackStateIndex, "desktop app must register the dialog plugin before managed playback state");
assert.match(appSource, /from "@tauri-apps\/plugin-dialog"/, "frontend must import the Tauri dialog plugin");
assert.match(appSource, /convertFileSrc/, "frontend must convert native paths into preview URLs");
assert.match(appSource, /openNativeMediaFiles/, "open control must use the native media picker");
assert.doesNotMatch(appSource, /fileInputRef/, "open control must not route through the hidden browser file input");
assert.match(appSource, /onDrop=/, "player shell must support drag-and-drop media loading");
assert.match(appSource, /togglePlayback/, "player shell must wire play and pause behavior");
assert.match(appSource, /seekTo/, "player shell must wire timeline seeking behavior");
assert.match(appSource, /setVolume/, "player shell must wire volume behavior");
assert.match(appSource, /type PlaybackSourceDto/, "frontend must define playback source DTO");
assert.match(appSource, /type MediaSourceKind = "localFilePath" \| "localFileLabel"/, "frontend must model native and preview-only media sources");
assert.match(appSource, /kind: "localFilePath" \| "localFileLabel" \| "localFolderLabel" \| "httpUrl"/, "frontend playback DTO must include localFilePath");
assert.match(appSource, /type PlaybackStatusDto/, "frontend must define playback status DTO");
assert.match(appSource, /type PlaybackSnapshotDto/, "frontend must define playback snapshot DTO");
assert.match(appSource, /type PlaybackCommandError/, "frontend must define playback command error DTO");
assert.match(appSource, /const \[queue, setQueue\]/, "frontend must keep queue state");
assert.match(appSource, /const \[currentIndex, setCurrentIndex\]/, "frontend must track the current queue index");
assert.match(appSource, /mediaItemFromNativePath/, "frontend must build queue items from native paths");
assert.match(appSource, /mediaItemFromBrowserFile/, "frontend must keep drag-and-drop preview file support");
assert.match(appSource, /function nextMediaItemId/, "frontend must assign fresh media item IDs for repeated selections");
assert.match(appSource, /id: nextMediaItemId\("native"\)/, "native media item IDs must not be derived from stable file paths");
assert.match(appSource, /id: nextMediaItemId\("preview"\)/, "preview media item IDs must not be derived from stable browser file metadata");
assert.match(appSource, /open\(\{[\s\S]*multiple:\s*true/, "native picker must allow selecting multiple files");
assert.match(appSource, /nativeOpenRequestIdRef/, "native picker must ignore stale dialog completions");
assert.match(appSource, /chooseQueueItem/, "playlist drawer must allow choosing queued files");
assert.match(appSource, /advanceToNextQueueItem/, "player must advance to the next queued file on media end");
assert.match(appSource, /pendingAutoplayRef/, "auto-advance must remember when to start the next item");
assert.match(appSource, /<video[\s\S]*key=\{media\.id\}/, "media element must remount when the current queue item changes");
assert.match(appSource, /onCanPlay=\{handleCanPlay\}/, "auto-advance playback must wait until the next preview can play");
assert.match(appSource, /playbackErrorMessage/, "frontend must normalize playback command errors");
assert.match(appSource, /runPlaybackCommand/, "frontend must use a playback command helper");
assert.match(appSource, /mirrorPlaybackCommand/, "frontend must mirror preview actions to playback commands");
assert.match(appSource, /playbackCommandIdRef/, "frontend must ignore stale playback command responses");
assert.match(appSource, /setPlaybackSnapshot\(snapshot\)/, "frontend must only store latest playback command snapshot");
assert.match(appSource, /commitSeekTo/, "frontend must commit backend seek separately from preview seek");
assert.match(appSource, /commitVolume/, "frontend must commit backend volume separately from preview volume");
assert.match(appSource, /role="alert"/, "playback errors must be announced to assistive technology");
for (const command of frontendPlaybackCommands) {
  assert.match(
    appSource,
    new RegExp(`invoke<PlaybackSnapshotDto>\\("${command}"|mirrorPlaybackCommand\\("${command}"`),
    `frontend must invoke ${command}`,
  );
}
assert.match(tauriGenerateHandler, /playback_snapshot/, "Tauri must register playback snapshot command");
assert.match(tauriGenerateHandler, /playback_open_preview_source/, "Tauri must register preview open command");
assert.match(tauriGenerateHandler, /playback_play/, "Tauri must register playback play command");
assert.match(tauriGenerateHandler, /playback_pause/, "Tauri must register playback pause command");
assert.match(tauriGenerateHandler, /playback_stop/, "Tauri must register playback stop command");
assert.match(tauriGenerateHandler, /playback_seek/, "Tauri must register playback seek command");
assert.match(tauriGenerateHandler, /playback_set_volume/, "Tauri must register playback volume command");
assert.match(tauriLibSource, /mod storage;/, "desktop app must include storage command module");
assert.match(tauriLibSource, /DesktopStorageState/, "desktop app must manage storage state");
for (const command of storageCommands) {
  assert.match(tauriGenerateHandler, new RegExp(command), `Tauri must register ${command}`);
}
assert.match(styles, /\.window-shell[\s\S]*border:\s*0/, "window shell must not draw an outer border");
assert.match(styles, /\.app-shell[\s\S]*padding:\s*0/, "window shell must not leave a transparent outer gutter");
assert.doesNotMatch(styles, /\.status-line/, "player should not reserve status text chrome over the video surface");
assert.match(styles, /playlist-item--active/, "playlist styles must mark the active queue item");
assert.match(mainSource, /windows_subsystem\s*=\s*"windows"/, "release Windows app must use GUI subsystem instead of opening a console");
assert.ok(
  capability.permissions.includes("core:window:allow-start-dragging"),
  "capability must allow Tauri start_dragging for whole-window drag",
);
assert.ok(capability.permissions.includes("core:window:allow-set-fullscreen"), "capability must allow fullscreen toggling");
assert.ok(capability.permissions.includes("core:window:allow-is-fullscreen"), "capability must allow reading fullscreen state");

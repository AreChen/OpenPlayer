import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const mainSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const tauriLibSource = await readFile(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
const capability = JSON.parse(await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
const workspaceToml = await readFile(new URL("../../../Cargo.toml", import.meta.url), "utf8");
const tauriCargoToml = await readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const tauriBuildScript = await readFile(new URL("../src-tauri/build.rs", import.meta.url), "utf8");

const [mainWindow] = config.app.windows;

assert.equal(config.app.windows.length, 1, "minimal shell should use one Tauri window from config");
assert.equal(mainWindow.url, "index.html", "packaged exe must load the bundled app entry");
assert.equal(mainWindow.decorations, false, "window must disable native decorations for custom titlebar");
assert.equal(mainWindow.transparent, false, "window must not use transparency that exposes an outer border");
assert.equal(config.app.security.csp, null, "minimal shell keeps the baseline CSP behavior");
assert.equal(config.app.security.assetProtocol, undefined, "minimal HTML playback must not expose Tauri asset protocol");
assert.match(config.build.devUrl, /23142$/, "Tauri dev URL must use the non-reserved Windows port");
assert.match(packageJson.scripts.dev, /23142$/, "Vite dev script must use the non-reserved Windows port");
assert.match(packageJson.scripts.preview, /23142$/, "Vite preview script must use the non-reserved Windows port");

assert.equal(packageJson.dependencies["movi-player"], undefined, "minimal branch must not ship WASM/software decoder dependency");
assert.equal(packageJson.dependencies["@tauri-apps/plugin-dialog"], undefined, "minimal branch must use browser File input instead of native dialog plugin");

assert.match(tauriCargoToml, /\[features\][\s\S]*mpv-smoke = \["dep:libmpv2"\]/, "libmpv2 spike must be hidden behind the mpv-smoke feature");
assert.match(tauriCargoToml, /libmpv2 = \{ version = "6\.0\.0", optional = true, default-features = false \}/, "libmpv2 must be optional and control-only for the first smoke spike");
assert.match(tauriBuildScript, /CARGO_FEATURE_MPV_SMOKE/, "build script must only add mpv link paths when mpv-smoke is enabled");
assert.match(tauriBuildScript, /vendor[\\/]native[\\/]mpv[\\/]windows-x64/, "build script must point at the vendored Windows mpv directory");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-smoke"\)\]\s*mod mpv_smoke;/, "desktop crate must keep libmpv2 smoke code feature-gated");
assert.doesNotMatch(appSource, /mpvSmoke|libmpv|libmpv2|mpv_smoke/, "libmpv2 smoke spike must not change the HTML video frontend path");

assert.match(appSource, /<video[\s\S]*ref=\{videoRef\}/, "player must render a native HTML video element");
assert.match(appSource, /fileInputRef/, "open control must use a hidden browser File input");
assert.match(appSource, /type="file"/, "browser file input must be present");
assert.match(appSource, /URL\.createObjectURL\(file\)/, "selected files must play through browser object URLs");
assert.match(appSource, /URL\.revokeObjectURL/, "object URLs must be revoked when queues are replaced or unmounted");
assert.match(appSource, /onChange=\{handleFileInputChange\}/, "file input must feed selected files into the queue");
assert.match(appSource, /onDrop=\{handleDrop\}/, "player shell must support drag-and-drop media loading");
assert.match(appSource, /togglePlayback/, "player shell must wire play and pause behavior");
assert.match(appSource, /video\.play\(\)/, "play control must call native video playback");
assert.match(appSource, /video\.pause\(\)/, "pause control must call native video playback");
assert.match(appSource, /video\.currentTime = value/, "seek control must update native video currentTime");
assert.match(appSource, /videoRef\.current\.volume = nextVolume/, "volume control must update native video volume");
assert.match(appSource, /advanceToNextQueueItem/, "player must advance through the selected queue");
assert.match(appSource, /pendingAutoplayRef/, "auto-advance must remember when to start the next item");

assert.doesNotMatch(appSource, /convertFileSrc|@tauri-apps\/plugin-dialog|openNativeMediaFiles/, "frontend must not use native path dialogs or asset URLs");
assert.doesNotMatch(appSource, /PlaybackSourceDto|PlaybackSnapshotDto|runPlaybackCommand|mirrorPlaybackCommand|storage_|recentMedia|PlaybackProgressDto/, "frontend must not keep backend playback, storage, recent, or progress plumbing");
assert.doesNotMatch(appSource, /movi-player|MoviPlayer|moviEventLog/, "frontend must not include Movi playback code");

assert.match(appSource, /getCurrentWindow/, "player must keep Tauri's imperative window API for fullscreen and drag");
assert.match(appSource, /startDragging/, "player surface must start native dragging from pointer events");
assert.match(appSource, /drag-surface/, "player must use an isolated drag layer behind controls");
assert.match(appSource, /onDoubleClick=\{toggleFullscreen\}/, "player surface double click must toggle fullscreen");
assert.match(appSource, /window_minimize/, "custom titlebar must wire minimize command");
assert.match(appSource, /window_toggle_maximize/, "custom titlebar must wire maximize command");
assert.match(appSource, /window_close/, "custom titlebar must wire close command");
assert.match(appSource, /playlist-drawer/, "playlist must remain a collapsible drawer");
assert.match(appSource, /togglePlaylist/, "control bar must keep playlist toggle");
assert.doesNotMatch(appSource, /titlebar-brand|titlebar-center|side-rail|status-line/, "confirmed baseline UI must not regress to the older chrome layout");

assert.match(styles, /\.window-shell[\s\S]*border:\s*0/, "window shell must not draw an outer border");
assert.match(styles, /\.app-shell[\s\S]*padding:\s*0/, "window shell must not leave a transparent outer gutter");
assert.doesNotMatch(styles, /\.recent-shortcuts|\.recent-drawer-section|\.status-line/, "minimal UI must not keep recent-media or status chrome styles");
assert.match(styles, /playlist-item--active/, "playlist styles must mark the active queue item");

assert.doesNotMatch(tauriLibSource, /mod playback|mod storage|tauri_plugin_dialog|DesktopPlaybackState|DesktopStorageState|playback_|storage_|openplayer_core|openplayer_shared/, "desktop backend must only keep minimal window commands");
assert.match(tauriLibSource, /window_minimize/, "desktop backend must keep minimize command");
assert.match(tauriLibSource, /window_toggle_maximize/, "desktop backend must keep maximize command");
assert.match(tauriLibSource, /window_close/, "desktop backend must keep close command");
assert.match(mainSource, /windows_subsystem\s*=\s*"windows"/, "release Windows app must use GUI subsystem instead of opening a console");

assert.ok(capability.permissions.includes("core:window:allow-start-dragging"), "capability must allow Tauri start_dragging for whole-window drag");
assert.ok(capability.permissions.includes("core:window:allow-set-fullscreen"), "capability must allow fullscreen toggling");
assert.ok(capability.permissions.includes("core:window:allow-is-fullscreen"), "capability must allow reading fullscreen state");
assert.ok(!capability.permissions.includes("dialog:allow-open"), "capability must not allow removed native file dialog");

assert.match(workspaceToml, /members = \[\s*"apps\/desktop\/src-tauri",\s*\]/, "workspace should only build the desktop shell crate");
assert.doesNotMatch(workspaceToml, /crates\//, "minimal workspace must not include old backend crates");

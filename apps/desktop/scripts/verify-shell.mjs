import assert from "node:assert/strict";
import { existsSync } from "node:fs";
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
const nsisHooksUrl = new URL("../src-tauri/nsis-hooks.nsh", import.meta.url);
const nsisHooksSource = existsSync(nsisHooksUrl) ? await readFile(nsisHooksUrl, "utf8") : "";
const mpvRenderUrl = new URL("../src-tauri/src/mpv_render.rs", import.meta.url);
const mpvRenderSource = existsSync(mpvRenderUrl) ? await readFile(mpvRenderUrl, "utf8") : "";
const mpvRenderSysUrl = new URL("../src-tauri/src/mpv_render/sys.rs", import.meta.url);
const mpvRenderSysSource = existsSync(mpvRenderSysUrl) ? await readFile(mpvRenderSysUrl, "utf8") : "";
const mpvRenderBackendSource = `${mpvRenderSource}\n${mpvRenderSysSource}`;
const embedCommandPattern = /mpv_embed_open_path|mpv_embed_play|mpv_embed_pause|mpv_embed_seek|mpv_embed_set_volume|mpv_embed_snapshot|mpv_embed_stop/;

function extractCfgFunction(source, cfgPattern, fnPattern) {
  const cfgMatch = cfgPattern.exec(source);
  if (!cfgMatch) {
    return "";
  }

  const afterCfg = cfgMatch.index + cfgMatch[0].length;
  const fnMatch = fnPattern.exec(source.slice(afterCfg));
  if (!fnMatch) {
    return "";
  }

  const fnStart = afterCfg + fnMatch.index;
  const openBrace = source.indexOf("{", fnStart + fnMatch[0].length);
  if (openBrace === -1) {
    return "";
  }

  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === "{") {
      depth += 1;
    } else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(fnStart, index + 1);
      }
    }
  }

  return "";
}

function extractFunctionAt(source, fnStart) {
  const openBrace = source.indexOf("{", fnStart);
  if (openBrace === -1) {
    return "";
  }

  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === "{") {
      depth += 1;
    } else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(fnStart, index + 1);
      }
    }
  }

  return "";
}

const mpvRenderRunMatch = /#\[cfg\(\s*feature\s*=\s*"mpv-render"\s*\)\]\s*pub\s+fn\s+run\s*\(\s*\)/.exec(tauriLibSource);
const mpvRenderRunSource = mpvRenderRunMatch
  ? extractFunctionAt(tauriLibSource, mpvRenderRunMatch.index + mpvRenderRunMatch[0].lastIndexOf("pub"))
  : "";

const [mainWindow] = config.app.windows;

assert.equal(config.app.windows.length, 1, "minimal shell should use one Tauri window from config");
assert.equal(mainWindow.url, "index.html?surface=video", "main window must be the video host surface");
assert.equal(mainWindow.decorations, false, "custom controls require custom window chrome once overlay is restored");
assert.equal(mainWindow.resizable, true, "main video window must remain freely resizable");
assert.equal(mainWindow.transparent, true, "video surface behind WebView requires transparent WebView/window composition");
assert.equal(config.app.security.csp, null, "minimal shell keeps the baseline CSP behavior");
assert.equal(config.app.security.assetProtocol, undefined, "minimal HTML playback must not expose Tauri asset protocol");
assert.equal(config.bundle.active, true, "desktop release builds must produce an installer bundle by default");
assert.equal(config.bundle.targets, "nsis", "desktop release build should target the Windows NSIS installer");
assert.ok(Object.keys(config.bundle.resources ?? {}).some((resource) => resource.endsWith("libmpv-2.dll")), "installer bundle must include the mpv runtime DLL");
assert.equal(config.bundle.windows?.nsis?.installerHooks, "nsis-hooks.nsh", "NSIS installer must install mpv runtime DLL next to the app executable");
assert.match(nsisHooksSource, /CopyFiles[\s\S]*libmpv-2\.dll[\s\S]*\$INSTDIR\\libmpv-2\.dll/, "NSIS hooks must copy libmpv-2.dll beside the installed executable");
assert.match(nsisHooksSource, /Delete[\s\S]*\$INSTDIR\\libmpv-2\.dll/, "NSIS hooks must remove copied mpv runtime DLL during uninstall");
assert.match(config.build.devUrl, /23142$/, "Tauri dev URL must use the non-reserved Windows port");
assert.match(packageJson.scripts.dev, /23142$/, "Vite dev script must use the non-reserved Windows port");
assert.match(packageJson.scripts.preview, /23142$/, "Vite preview script must use the non-reserved Windows port");

assert.equal(packageJson.dependencies["movi-player"], undefined, "minimal branch must not ship WASM/software decoder dependency");
assert.ok(packageJson.dependencies["@tauri-apps/plugin-dialog"], "mpv path playback must use Tauri dialog to obtain real local paths");

assert.match(tauriCargoToml, /default = \["mpv-render"\]/, "desktop default feature must use the mpv render backend");
assert.match(tauriCargoToml, /mpv-render/, "Cargo features must define mpv-render");
assert.match(tauriBuildScript, /CARGO_FEATURE_MPV_SMOKE[\s\S]*CARGO_FEATURE_MPV_EMBED/, "build script must only add mpv link paths when an mpv feature is enabled");
assert.match(tauriBuildScript, /vendor[\\/]native[\\/]mpv[\\/]windows-x64/, "build script must point at the vendored Windows mpv directory");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-smoke"\)\]\s*mod mpv_smoke;/, "desktop crate must keep libmpv2 smoke code feature-gated");
assert.match(tauriLibSource, /mod mpv_embed;/, "overlay fallback must use mpv child HWND in the main video window");
assert.match(mpvRenderRunSource, /pub\s+fn\s+run\s*\(\s*\)/, "desktop default runtime must include an mpv-render run function");
assert.match(mpvRenderRunSource, /WebviewWindowBuilder[\s\S]*surface=overlay/, "desktop runtime must create a separate transparent overlay controls window");
assert.match(mpvRenderRunSource, /mpv_overlay_open_path/, "default runtime must register overlay commands that target the main video window");
assert.doesNotMatch(mpvRenderRunSource, /\.always_on_top\(true\)/, "overlay controls must not be globally topmost over other apps");
assert.match(tauriLibSource, /GWLP_HWNDPARENT|set_overlay_owner/, "overlay controls should be owned by the main player window instead of global topmost");
assert.doesNotMatch(mpvRenderRunSource, /OPENPLAYER_MPV_EMBED_FILE/, "normal render API runtime must not auto-play the old Abbott embed smoke file");
assert.match(tauriLibSource, /tauri_plugin_dialog::init\(\)/, "desktop app must register Tauri dialog plugin for path-based mpv playback");
assert.ok(capability.permissions.includes("dialog:allow-open"), "capability must allow file-open dialog for path-based mpv playback");
assert.ok(capability.windows.includes("overlay"), "overlay controls window must be included in the dialog capability scope");
assert.doesNotMatch(appSource, /<video\b|URL\.createObjectURL/, "mpv-first player must not use browser video or object URLs");
assert.match(appSource, /surface=video|surface === "video"/, "frontend must render a video-only main surface separately from overlay controls");
assert.match(appSource, /open\(/, "player must keep native Tauri file picker access");
assert.match(appSource, /isPickerOpen/, "native file picker must guard against repeated open calls while the dialog is pending");
assert.match(appSource, /mpv_overlay_open_path/, "overlay controls must open files through commands targeting the main video window");
assert.match(appSource, /@tauri-apps\/plugin-dialog/, "frontend must use native dialog for mpv path selection");
assert.match(appSource, /openNativeMediaFiles/, "frontend must expose a picker-driven mpv open action");
assert.doesNotMatch(appSource, /mpvSmoke|libmpv|libmpv2|mpv_smoke/, "libmpv2 smoke spike must not change the HTML video frontend path");
assert.match(mpvRenderBackendSource, /mpv_render_context_create|create_render_context/, "mpv render backend must create an mpv render context");
assert.match(mpvRenderBackendSource, /MPV_RENDER_API_TYPE_OPENGL|RenderParamApiType::OpenGl/, "mpv render backend must use the OpenGL render API");
assert.doesNotMatch(mpvRenderBackendSource, /set_option\("wid"|set_option_string\("wid"|MPV_RENDER_PARAM_X11_DISPLAY/, "mpv render backend must not use mpv-owned native window embedding");

assert.doesNotMatch(appSource, /<video|videoRef|fileInputRef|type="file"|URL\.createObjectURL|URL\.revokeObjectURL|handleFileInputChange|onDrop=\{handleDrop\}/, "mpv-first player must not keep HTML video or browser File playback");
assert.match(appSource, /togglePlayback/, "player shell must wire play and pause behavior");
assert.match(appSource, /window_toggle_fullscreen/, "overlay UI must route fullscreen through a backend command targeting the main video window");
assert.doesNotMatch(appSource, /getCurrentWindow\(\)[\s\S]*setFullscreen/, "overlay UI must not fullscreen the overlay window itself");
assert.match(appSource, /event\.button\s*===\s*1[\s\S]*window_toggle_fullscreen/, "non-control surface must toggle fullscreen with the middle mouse button");
assert.doesNotMatch(appSource, /video\.play\(\)|video\.pause\(\)|video\.currentTime|videoRef\.current\.volume|advanceToNextQueueItem|pendingAutoplayRef/, "controls must not call HTML video APIs");

assert.doesNotMatch(appSource, /convertFileSrc/, "frontend must not use asset URLs or old native preview plumbing");
assert.doesNotMatch(appSource, /PlaybackSourceDto|PlaybackSnapshotDto|runPlaybackCommand|mirrorPlaybackCommand|storage_|recentMedia|PlaybackProgressDto/, "frontend must not keep backend playback, storage, recent, or progress plumbing");
assert.doesNotMatch(appSource, /movi-player|MoviPlayer|moviEventLog/, "frontend must not include Movi playback code");

assert.match(appSource, /playlist-drawer/, "playlist must remain a collapsible drawer");
assert.match(appSource, /togglePlaylist/, "control bar must keep playlist toggle");
assert.match(appSource, /data-tauri-drag-region/, "custom chrome must use Tauri drag-region markup instead of JS pointer dragging");
assert.match(appSource, /window_start_drag/, "overlay drag region must ask the backend to drag the main video window");
assert.doesNotMatch(appSource, /onDoubleClick=\{toggleFullscreen\}/, "fullscreen must use middle mouse button instead of double-click");
assert.match(appSource, /window_start_resize/, "overlay resize handles must ask the backend to resize the main video window");
assert.match(appSource, /pendingSeek/, "seek UI must track pending seek targets to prevent stale snapshot rollback");
assert.match(appSource, /SEEK_CONFIRM_TOLERANCE_SECONDS/, "seek UI must define a tolerance for mpv seek confirmation");
assert.match(appSource, /SEEK_SNAPSHOT_SUPPRESS_MS/, "seek UI must bound stale snapshot suppression while mpv catches up");
assert.doesNotMatch(appSource, /setPointerCapture|releasePointerCapture|startDragging|DragIntent|continueWindowDragIntent|beginWindowDragIntent/, "custom chrome must not use pointer-capture startDragging loop that freezes render windows");
assert.doesNotMatch(appSource, /titlebar-brand|titlebar-center|side-rail|status-line/, "confirmed baseline UI must not regress to the older chrome layout");

assert.match(styles, /\.window-shell[\s\S]*border:\s*0/, "window shell must not draw an outer border");
assert.match(styles, /\.app-shell[\s\S]*padding:\s*0/, "window shell must not leave a transparent outer gutter");
assert.doesNotMatch(styles, /\.recent-shortcuts|\.recent-drawer-section|\.status-line/, "minimal UI must not keep recent-media or status chrome styles");
assert.match(styles, /playlist-item--active/, "playlist styles must mark the active queue item");
assert.match(styles, /\.drag-region\s*\{[\s\S]*inset:\s*0/, "drag region must cover the non-control player surface, not just the title strip");
assert.match(styles, /\.resize-region/, "overlay must expose explicit resize hit areas around the border");
assert.match(styles, /\.resize-region--south-east/, "overlay must include corner resize hit areas");
assert.match(styles, /\.transport\s*\{[\s\S]*pointer-events:\s*auto/, "transport controls must remain interactive above the full-surface drag region");
assert.match(styles, /\.playlist-drawer--open\s*\{[\s\S]*pointer-events:\s*auto/, "open playlist drawer must remain interactive above the full-surface drag region");

assert.doesNotMatch(tauriLibSource, /mod playback|mod storage|DesktopPlaybackState|DesktopStorageState|playback_|storage_|openplayer_core|openplayer_shared/, "desktop backend must not restore removed playback or storage plumbing");
assert.match(tauriLibSource, /window_minimize/, "desktop backend must keep minimize command");
assert.match(tauriLibSource, /window_toggle_maximize/, "desktop backend must keep maximize command");
assert.match(tauriLibSource, /window_toggle_fullscreen[\s\S]*main_window\(&app\)\?[\s\S]*set_fullscreen/, "desktop backend must toggle fullscreen on the main video window");
assert.match(tauriLibSource, /window_start_resize[\s\S]*start_resize_dragging/, "desktop backend must start resizing the main video window from overlay hit areas");
assert.match(tauriLibSource, /window_close/, "desktop backend must keep close command");
assert.match(tauriLibSource, /window_start_drag[\s\S]*main_window\(&app\)\?[\s\S]*start_dragging/, "backend must drag the main video window when overlay drag strip is used");
assert.match(mainSource, /windows_subsystem\s*=\s*"windows"/, "release Windows app must use GUI subsystem instead of opening a console");

assert.ok(!capability.permissions.includes("core:window:allow-start-dragging"), "capability must not allow the removed JS start_dragging path");
assert.ok(capability.permissions.includes("core:window:allow-set-fullscreen"), "capability must allow fullscreen toggling");
assert.ok(capability.permissions.includes("core:window:allow-is-fullscreen"), "capability must allow reading fullscreen state");
assert.ok(capability.permissions.includes("dialog:allow-open"), "capability must allow native file dialog for mpv path playback");

assert.match(workspaceToml, /members = \[\s*"apps\/desktop\/src-tauri",\s*\]/, "workspace should only build the desktop shell crate");
assert.doesNotMatch(workspaceToml, /crates\//, "minimal workspace must not include old backend crates");

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const windowsConfigUrl = new URL("../src-tauri/tauri.windows.conf.json", import.meta.url);
const windowsConfig = existsSync(windowsConfigUrl) ? JSON.parse(await readFile(windowsConfigUrl, "utf8")) : {};
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const indexHtml = await readFile(new URL("../index.html", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const mainSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const tauriLibSource = await readFile(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
const playbackStoreUrl = new URL("../src-tauri/src/playback_store.rs", import.meta.url);
const playbackStoreSource = existsSync(playbackStoreUrl) ? await readFile(playbackStoreUrl, "utf8") : "";
const capability = JSON.parse(await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
const ciWorkflow = await readFile(new URL("../../../.github/workflows/ci.yml", import.meta.url), "utf8");
const releaseWorkflowUrl = new URL("../../../.github/workflows/release.yml", import.meta.url);
const releaseWorkflow = existsSync(releaseWorkflowUrl) ? await readFile(releaseWorkflowUrl, "utf8") : "";
const releaseVerifyScriptUrl = new URL("./verify-release.mjs", import.meta.url);
const releaseMpvManifestUrl = new URL("../../../docs/native-deps/mpv-windows-x64.json", import.meta.url);
const workspaceToml = await readFile(new URL("../../../Cargo.toml", import.meta.url), "utf8");
const tauriCargoToml = await readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const tauriBuildScript = await readFile(new URL("../src-tauri/build.rs", import.meta.url), "utf8");
const nsisHooksUrl = new URL("../src-tauri/nsis-hooks.nsh", import.meta.url);
const nsisHooksSource = existsSync(nsisHooksUrl) ? await readFile(nsisHooksUrl, "utf8") : "";
const mpvEmbedUrl = new URL("../src-tauri/src/mpv_embed.rs", import.meta.url);
const mpvEmbedSource = existsSync(mpvEmbedUrl) ? await readFile(mpvEmbedUrl, "utf8") : "";
const mpvRenderFiles = [
  new URL("../src-tauri/src/mpv_render.rs", import.meta.url),
  new URL("../src-tauri/src/mpv_render/sys.rs", import.meta.url),
  new URL("../src-tauri/src/mpv_render/win32_surface.rs", import.meta.url),
];
const rootLogoUrl = new URL("../../../openplayer_logo_10001000.png", import.meta.url);
const uiLogoUrl = new URL("../src/assets/openplayer-logo.png", import.meta.url);
const tauriIconPngUrl = new URL("../src-tauri/icons/icon.png", import.meta.url);
const tauriIconIcoUrl = new URL("../src-tauri/icons/icon.ico", import.meta.url);
const embedCommandPattern = /mpv_embed_open_path|mpv_embed_play|mpv_embed_pause|mpv_embed_seek|mpv_embed_set_volume|mpv_embed_set_speed|mpv_embed_select_track|mpv_embed_add_subtitle|mpv_embed_snapshot|mpv_embed_stop/;

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

const mpvEmbedRunMatch = /#\[cfg\(\s*feature\s*=\s*"mpv-embed"\s*\)\]\s*pub\s+fn\s+run\s*\(\s*\)/.exec(tauriLibSource);
const mpvEmbedRunSource = mpvEmbedRunMatch
  ? extractFunctionAt(tauriLibSource, mpvEmbedRunMatch.index + mpvEmbedRunMatch[0].lastIndexOf("pub"))
  : "";
const windowToggleFullscreenMatch = /#\[tauri::command\]\s*fn\s+window_toggle_fullscreen\s*\(/.exec(tauriLibSource);
const windowToggleFullscreenSource = windowToggleFullscreenMatch
  ? extractFunctionAt(tauriLibSource, windowToggleFullscreenMatch.index + windowToggleFullscreenMatch[0].lastIndexOf("fn"))
  : "";

const [mainWindow] = config.app.windows;

assert.equal(config.app.windows.length, 1, "minimal shell should use one Tauri window from config");
assert.equal(mainWindow.url, "index.html?surface=video", "main window must be the video host surface");
assert.equal(mainWindow.decorations, false, "custom controls require custom window chrome once overlay is restored");
assert.equal(mainWindow.resizable, true, "main video window must remain freely resizable");
assert.equal(mainWindow.transparent, true, "video surface behind WebView requires transparent WebView/window composition");
assert.equal(config.app.security.csp, null, "minimal shell keeps the baseline CSP behavior");
assert.equal(config.app.security.assetProtocol, undefined, "minimal HTML playback must not expose Tauri asset protocol");
assert.equal(config.bundle?.resources, undefined, "base Tauri config must not require ignored Windows mpv DLL resources during Linux CI");
assert.equal(windowsConfig.bundle?.active, true, "Windows desktop release builds must produce an installer bundle by default");
assert.equal(windowsConfig.bundle?.targets, "nsis", "Windows desktop release build should target the NSIS installer");
assert.ok(Object.keys(windowsConfig.bundle?.resources ?? {}).some((resource) => resource.endsWith("libmpv-2.dll")), "Windows installer bundle must include the mpv runtime DLL");
assert.equal(windowsConfig.bundle?.windows?.nsis?.installerHooks, "nsis-hooks.nsh", "NSIS installer must install mpv runtime DLL next to the app executable");
assert.match(nsisHooksSource, /CopyFiles[\s\S]*libmpv-2\.dll[\s\S]*\$INSTDIR\\libmpv-2\.dll/, "NSIS hooks must copy libmpv-2.dll beside the installed executable");
assert.match(nsisHooksSource, /Delete[\s\S]*\$INSTDIR\\libmpv-2\.dll/, "NSIS hooks must remove copied mpv runtime DLL during uninstall");
assert.match(config.build.devUrl, /23142$/, "Tauri dev URL must use the non-reserved Windows port");
assert.match(packageJson.scripts.dev, /23142$/, "Vite dev script must use the non-reserved Windows port");
assert.match(packageJson.scripts.preview, /23142$/, "Vite preview script must use the non-reserved Windows port");
assert.equal(packageJson.scripts["verify:release"], "node scripts/verify-release.mjs", "desktop package must expose release metadata verification");
assert.ok(existsSync(releaseVerifyScriptUrl), "release verification script must exist");
assert.ok(existsSync(releaseMpvManifestUrl), "Windows mpv dependency manifest must be tracked outside ignored vendor binaries");
assert.match(ciWorkflow, /npm run verify:release/, "CI must verify release metadata");
assert.match(ciWorkflow, /npm run verify:shell/, "CI must run shell structural verification");
assert.match(ciWorkflow, /FORCE_JAVASCRIPT_ACTIONS_TO_NODE24:\s*true/, "CI must opt into the GitHub Actions Node 24 runtime");
assert.doesNotMatch(`${ciWorkflow}\n${releaseWorkflow}`, /actions\/(?:checkout|setup-node)@v4/, "GitHub workflows must not use deprecated Node 20 action major versions");
assert.match(releaseWorkflow, /on:[\s\S]*tags:[\s\S]*v\*/, "release workflow must run for version tags");
assert.match(releaseWorkflow, /FORCE_JAVASCRIPT_ACTIONS_TO_NODE24:\s*true/, "release workflow must opt into the GitHub Actions Node 24 runtime");
assert.match(releaseWorkflow, /npm run tauri:build -- --config src-tauri\/tauri\.windows\.conf\.json/, "release workflow must build the Windows NSIS installer");
assert.match(releaseWorkflow, /mpv-windows-x64\.json/, "release workflow must restore mpv runtime from tracked dependency metadata");
assert.match(releaseWorkflow, /Get-FileHash[\s\S]*SHA256/, "release workflow must generate a SHA256 checksum for the installer");
assert.match(releaseWorkflow, /gh release/, "release workflow must publish installer assets to GitHub Releases");
assert.match(indexHtml, /surface["')\s.]*={1,3}\s*"video"[\s\S]*surface-video/, "index.html must classify the main video surface before React mounts");
assert.match(indexHtml, /surface-overlay/, "index.html must classify non-video surfaces as transparent overlays before React mounts");
assert.match(indexHtml, /html\.surface-video[\s\S]*background:\s*#000/, "video surface must paint black before React and mpv finish loading");
assert.match(indexHtml, /html\.surface-overlay[\s\S]*background:\s*transparent/, "overlay surface must remain transparent before React mounts");
assert.ok(!existsSync(rootLogoUrl), "source logo must live under app assets, not the repository root");
assert.ok(existsSync(uiLogoUrl), "frontend logo asset must exist");
assert.ok(existsSync(tauriIconPngUrl), "Tauri PNG icon must exist");
assert.ok(existsSync(tauriIconIcoUrl), "Windows ICO icon must exist");

assert.equal(packageJson.dependencies["movi-player"], undefined, "minimal branch must not ship WASM/software decoder dependency");
assert.ok(packageJson.dependencies["@tauri-apps/plugin-dialog"], "mpv path playback must use Tauri dialog to obtain real local paths");

assert.match(tauriCargoToml, /default = \["mpv-embed"\]/, "desktop default feature must use the stable mpv embed overlay backend");
assert.match(ciWorkflow, /libmpv-dev/, "Linux CI must install libmpv-dev so default mpv-embed tests can link libmpv");
assert.match(tauriCargoToml, /mpv-embed/, "Cargo features must define mpv-embed");
assert.doesNotMatch(tauriCargoToml, /mpv-render|libmpv2-sys|Win32_Graphics_OpenGL/, "desktop crate must not keep the failed mpv render backend or its render-only dependencies");
assert.doesNotMatch(tauriBuildScript, /CARGO_FEATURE_MPV_RENDER/, "build script must not keep the removed mpv-render feature gate");
assert.match(tauriBuildScript, /CARGO_FEATURE_MPV_SMOKE[\s\S]*CARGO_FEATURE_MPV_EMBED/, "build script must only add mpv link paths when an mpv feature is enabled");
assert.match(tauriBuildScript, /vendor[\\/]native[\\/]mpv[\\/]windows-x64/, "build script must point at the vendored Windows mpv directory");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-smoke"\)\]\s*mod mpv_smoke;/, "desktop crate must keep libmpv2 smoke code feature-gated");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-embed"\)\]\s*mod mpv_embed;/, "desktop crate must compile the stable mpv child HWND backend behind mpv-embed");
assert.doesNotMatch(tauriLibSource, /mpv_render|MpvRenderState|mpv_render_/, "desktop runtime must not reference the removed mpv render backend");
for (const fileUrl of mpvRenderFiles) {
  assert.ok(!existsSync(fileUrl), `removed render spike file must not exist: ${fileUrl.pathname}`);
}
assert.match(mpvEmbedRunSource, /pub\s+fn\s+run\s*\(\s*\)/, "desktop default runtime must include an mpv-embed run function");
assert.match(mpvEmbedRunSource, /WebviewWindowBuilder[\s\S]*surface=overlay/, "desktop runtime must create a separate transparent overlay controls window");
assert.match(mpvEmbedRunSource, /mpv_overlay_open_path/, "default runtime must register overlay commands that target the main video window");
assert.match(tauriLibSource, /mpv_overlay_open_path[\s\S]*main_window\(&app\)\?[\s\S]*mpv_embed::open_path_for_window\(&main, state\.inner\(\), path\)/, "overlay open command must target the main video window through mpv_embed");
assert.doesNotMatch(mpvEmbedRunSource, /\.always_on_top\(true\)/, "overlay controls must not be globally topmost over other apps");
assert.doesNotMatch(mpvEmbedRunSource, /\.position\(position\.x as f64, position\.y as f64\)|\.inner_size\(size\.width as f64, size\.height as f64\)/, "overlay startup must not pass physical main window pixels to logical builder sizing APIs");
assert.match(mpvEmbedRunSource, /\.visible\(false\)[\s\S]*sync_overlay_to_main\(&app_handle\)[\s\S]*overlay\.show\(\)/, "overlay startup must stay hidden until physical-position sync prevents DPI-scale misalignment");
assert.match(tauriLibSource, /#\[cfg\(all\(feature = "mpv-embed", windows\)\)\][\s\S]*SetWindowLongPtrW/, "overlay HWND ownership must be isolated to Windows mpv-embed builds");
assert.match(tauriLibSource, /#\[cfg\(all\(feature = "mpv-embed", not\(windows\)\)\)\][\s\S]*fn set_overlay_owner/, "non-Windows mpv-embed builds must not call Windows overlay ownership APIs");
assert.doesNotMatch(mpvEmbedRunSource, /OPENPLAYER_MPV_EMBED_FILE/, "normal embed overlay runtime must not auto-play the old Abbott embed smoke file");
assert.match(tauriLibSource, /tauri_plugin_dialog::init\(\)/, "desktop app must register Tauri dialog plugin for path-based mpv playback");
assert.ok(capability.permissions.includes("dialog:allow-open"), "capability must allow file-open dialog for path-based mpv playback");
assert.ok(capability.windows.includes("overlay"), "overlay controls window must be included in the dialog capability scope");
assert.doesNotMatch(appSource, /<video\b|URL\.createObjectURL/, "mpv-first player must not use browser video or object URLs");
assert.match(appSource, /surface=video|surface === "video"/, "frontend must render a video-only main surface separately from overlay controls");
assert.match(appSource, /open\(/, "player must keep native Tauri file picker access");
assert.match(appSource, /openplayer-logo\.png/, "empty player state must use the OpenPlayer logo asset");
assert.doesNotMatch(appSource, /MPV native playback/, "empty player state must not show the old playback engine tagline");
assert.match(appSource, /isPickerOpen/, "native file picker must guard against repeated open calls while the dialog is pending");
assert.match(appSource, /mpv_overlay_open_path/, "overlay controls must open files through commands targeting the main video window");
assert.match(appSource, /@tauri-apps\/plugin-dialog/, "frontend must use native dialog for mpv path selection");
assert.match(appSource, /openNativeMediaFiles/, "frontend must expose a picker-driven mpv open action");
assert.match(appSource, /type TimeDisplayMode\s*=\s*"timecode"\s*\|\s*"frames"/, "frontend must define a timecode/frame display mode");
assert.match(appSource, /type PlaybackClockAnchor/, "frontend must keep a display-clock anchor for smooth progress interpolation");
assert.match(appSource, /requestAnimationFrame/, "frontend must animate displayed progress with requestAnimationFrame");
assert.match(appSource, /anchorDisplayClock/, "frontend must reset the smooth display clock when mpv state changes");
assert.match(appSource, /formatTimecode/, "frontend must use adaptive timecode formatting");
assert.match(appSource, /formatFrameCount/, "frontend must format frame counts for frame mode");
assert.match(appSource, /toggleTimeDisplayMode/, "transport time labels must toggle timecode and frame display modes");
assert.match(appSource, /const displayTime\s*=\s*snapEndOfMediaPosition\(displayPosition/, "seek slider must use the interpolated display position");
assert.match(appSource, /Math\.floor\(displayTime \* framesPerSecond\)/, "current frame must be derived from smooth display time and fps");
assert.match(appSource, /Math\.floor\(duration \* framesPerSecond\)/, "total frame count must be derived from duration and fps");
assert.match(appSource, /fps:\s*number/, "frontend snapshot type must include fps metadata");
assert.match(appSource, /speed:\s*number/, "frontend snapshot type must include playback speed metadata");
assert.match(appSource, /subtitleDelay:\s*number/, "frontend snapshot type must include subtitle delay metadata");
assert.match(appSource, /type MpvTrack/, "frontend must define mpv track metadata for audio, video, and subtitles");
assert.match(appSource, /playbackSpeedOptions/, "frontend must expose curated playback speed choices");
assert.match(appSource, /mpv_embed_set_subtitle_delay/, "frontend must control mpv subtitle delay through a backend command");
assert.match(appSource, /subtitle-delay/, "media options panel must expose subtitle delay controls");
assert.doesNotMatch(appSource, /mpvSmoke|libmpv|libmpv2|mpv_smoke/, "libmpv2 smoke spike must not change the HTML video frontend path");
assert.doesNotMatch(appSource, /<video|videoRef|fileInputRef|type="file"|URL\.createObjectURL|URL\.revokeObjectURL|handleFileInputChange|onDrop=\{handleDrop\}/, "mpv-first player must not keep HTML video or browser File playback");
assert.match(appSource, /togglePlayback/, "player shell must wire play and pause behavior");
assert.match(appSource, /type ShortcutAction/, "player shell must define configurable shortcut actions");
assert.match(appSource, /defaultShortcutBindings/, "player shell must define default shortcut bindings");
assert.match(appSource, /OPENPLAYER_SHORTCUTS_STORAGE_KEY/, "shortcut settings must persist through localStorage");
assert.match(appSource, /openplayer\.shortcuts\.v3/, "shortcut storage version must reset stale D/F and fullscreen bindings");
assert.doesNotMatch(appSource, /openplayer\.shortcuts\.v2/, "shortcut storage must not keep the stale v2 bindings");
assert.match(appSource, /type PlaybackHistoryEntry/, "player shell must define typed playback history entries");
assert.match(appSource, /history_list/, "player shell must read persisted playback history through a backend command");
assert.match(appSource, /history_remember/, "player shell must write playback history through a backend command");
assert.match(appSource, /history_resume_position/, "player shell must resolve resume positions through a backend command");
assert.match(appSource, /resumePositionForPath/, "player shell must resume remembered playback positions when opening known paths");
assert.match(appSource, /rememberPlaybackProgress/, "player shell must remember progress from mpv snapshots");
assert.match(appSource, /RESUME_END_PROGRESS_RATIO/, "resume logic must use a duration-relative end threshold");
assert.match(appSource, /MIN_RESUME_PROGRESS_RATIO/, "resume logic must use a duration-relative start threshold");
assert.doesNotMatch(appSource, /OPENPLAYER_HISTORY_STORAGE_KEY|openplayer\.history\.v1|readPlaybackHistory|writePlaybackHistory|window\.localStorage\.setItem\(OPENPLAYER_HISTORY_STORAGE_KEY/, "playback history must not remain backed by frontend localStorage");
assert.match(appSource, /keyboardEventToChord/, "shortcut settings must normalize keyboard events into configurable chords");
assert.match(appSource, /performShortcutAction/, "global shortcut dispatch must route configured chords to player commands");
assert.match(appSource, /recordingShortcutAction/, "settings must support recording a replacement shortcut chord");
assert.match(appSource, /shortcutKeyDownRef/, "global keydown dispatch must use a stable ref so playback progress animation does not reinstall shortcut listeners");
assert.match(appSource, /window\.addEventListener\("keydown",\s*handleGlobalKeyDown,\s*\{\s*capture:\s*true\s*\}\);\s*return[\s\S]*window\.removeEventListener\("keydown",\s*handleGlobalKeyDown,\s*\{\s*capture:\s*true\s*\}\);\s*},\s*\[\]\)/, "global keydown listener must be installed once and read current state through refs");
assert.match(appSource, /openplayer-native-shortcut/, "frontend must listen for native shortcut events when the overlay webview is not focused");
assert.match(appSource, /window_update_shortcuts/, "frontend must mirror custom shortcut bindings to the native shortcut bridge");
assert.match(appSource, /window_set_shortcuts_enabled/, "frontend must disable native shortcut dispatch while menus or shortcut capture are active");
assert.match(appSource, /frameForward/, "player shell must define a configurable forward-one-frame shortcut action");
assert.match(appSource, /frameBackward/, "player shell must define a configurable backward-one-frame shortcut action");
assert.match(appSource, /toggleFullscreen:\s*"Enter"/, "fullscreen shortcut must default to Enter");
assert.match(appSource, /frameBackward:\s*"D"/, "backward-one-frame shortcut must default to D");
assert.match(appSource, /frameForward:\s*"F"/, "forward-one-frame shortcut must default to F");
assert.match(appSource, /mpv_embed_frame_step/, "forward-one-frame shortcut must use mpv frame-step");
assert.match(appSource, /mpv_embed_frame_back_step/, "backward-one-frame shortcut must use mpv frame-back-step");
assert.match(appSource, /mpv_embed_set_speed/, "frontend must control mpv playback speed through a backend command");
assert.match(appSource, /mpv_embed_select_track/, "frontend must switch audio, video, and subtitle tracks through a backend command");
assert.match(appSource, /mpv_embed_add_subtitle/, "frontend must load external subtitle files through mpv");
assert.doesNotMatch(appSource, /closest\([^)]*button|role='button'/, "global shortcuts must not be disabled just because a player button is focused");
assert.match(appSource, /isTextEntryShortcutTarget/, "global shortcut filtering must only suppress text-entry targets");
assert.match(appSource, /TEXT_ENTRY_INPUT_TYPES/, "global shortcut filtering must not suppress range sliders or player buttons");
assert.match(appSource, /releaseShortcutFocusTarget/, "global shortcuts must release non-text control focus before dispatching player actions");
assert.match(appSource, /window_focus_overlay/, "frontend must be able to restore overlay focus after native dialogs or window commands");
assert.match(appSource, /context-menu/, "overlay must render a custom right-click context menu");
assert.match(appSource, /settings-dialog/, "overlay must render a settings dialog for player preferences");
assert.match(appSource, /onContextMenu=\{openContextMenu\}/, "player surface must open the custom context menu on right click");
assert.match(appSource, /window_toggle_fullscreen/, "overlay UI must route fullscreen through a backend command targeting the main video window");
assert.doesNotMatch(appSource, /getCurrentWindow\(\)[\s\S]*setFullscreen/, "overlay UI must not fullscreen the overlay window itself");
assert.match(appSource, /event\.button\s*===\s*1[\s\S]*window_toggle_fullscreen/, "non-control surface must toggle fullscreen with the middle mouse button");
assert.doesNotMatch(appSource, /video\.play\(\)|video\.pause\(\)|video\.currentTime|videoRef\.current\.volume|advanceToNextQueueItem|pendingAutoplayRef/, "controls must not call HTML video APIs");

assert.doesNotMatch(appSource, /convertFileSrc/, "frontend must not use asset URLs or old native preview plumbing");
assert.doesNotMatch(appSource, /PlaybackSourceDto|PlaybackSnapshotDto|runPlaybackCommand|mirrorPlaybackCommand|storage_|recentMedia|PlaybackProgressDto/, "frontend must not keep backend playback, storage, recent, or progress plumbing");
assert.doesNotMatch(appSource, /movi-player|MoviPlayer|moviEventLog/, "frontend must not include Movi playback code");

assert.match(appSource, /playlist-drawer/, "playlist must remain a collapsible drawer");
assert.match(appSource, /togglePlaylist/, "control bar must keep playlist toggle");
assert.match(appSource, /history-section/, "playlist drawer must expose local playback history");
assert.match(appSource, /openHistoryEntry/, "playback history entries must reopen media paths");
assert.match(appSource, /data-tauri-drag-region/, "custom chrome must use Tauri drag-region markup instead of JS pointer dragging");
assert.match(appSource, /window_start_drag/, "overlay drag region must ask the backend to drag the main video window");
assert.doesNotMatch(appSource, /onDoubleClick=\{toggleFullscreen\}/, "fullscreen must use middle mouse button instead of double-click");
assert.match(appSource, /window_start_resize/, "overlay resize handles must ask the backend to resize the main video window");
assert.match(appSource, /pendingSeek/, "seek UI must track pending seek targets to prevent stale snapshot rollback");
assert.match(appSource, /SEEK_CONFIRM_TOLERANCE_SECONDS/, "seek UI must define a tolerance for mpv seek confirmation");
assert.match(appSource, /SEEK_SNAPSHOT_SUPPRESS_MS/, "seek UI must bound stale snapshot suppression while mpv catches up");
assert.match(appSource, /AUTO_HIDE_CONTROLS_MS\s*=\s*5000/, "overlay chrome must hide after 5 seconds of no user activity");
assert.match(appSource, /stage--chrome-hidden/, "overlay must apply a hidden chrome class after inactivity");
assert.match(appSource, /onPointerMove=\{recordUserActivity\}/, "overlay must reveal controls on pointer movement");
assert.match(appSource, /handleShellPointerLeave/, "overlay must hide player chrome when the pointer leaves the window");
assert.match(appSource, /onPointerLeave=\{handleShellPointerLeave\}/, "player shell must listen for pointer leave to hide inactive chrome");
assert.match(appSource, /isChromePinned/, "overlay must pin controls while dialogs, errors, or drawers are active");
assert.match(appSource, /snapshot\.status\s*===\s*"playing"/, "frontend must only animate the display clock for explicit playing snapshots");
assert.doesNotMatch(appSource, /snapshot\.status\s*!==\s*"idle"\s*&&\s*snapshot\.status\s*!==\s*"ended"/, "frontend must not treat paused frame-step snapshots as playing");
assert.match(appSource, /END_OF_MEDIA_SNAP_TOLERANCE_SECONDS/, "frontend must define a near-end snap tolerance for the visible slider value");
assert.match(appSource, /snapEndOfMediaPosition/, "frontend must snap visible end-of-media slider value to duration");
assert.match(appSource, /const displayTime\s*=\s*snapEndOfMediaPosition/, "frontend must derive a snapped display time for labels, progress, and range value");
assert.match(appSource, /value=\{displayTime\}/, "seek slider thumb must use the snapped display time, not raw currentTime");
assert.match(appSource, /className="seek-slider"[\s\S]*step="any"/, "seek slider must use step=any so short clip durations can render exactly at max");
assert.match(appSource, /className="seek-control"[\s\S]*--progress/, "seek UI must expose a stable progress wrapper for smooth visual fill");
assert.match(appSource, /className="seek-progress"/, "seek UI must render an independent progress fill instead of relying only on native range background repainting");
assert.match(appSource, /--progress-ratio/, "seek UI must expose a unitless progress ratio for exact custom thumb positioning");
assert.match(appSource, /className="seek-thumb"/, "seek UI must render its own thumb instead of relying on WebView range pseudo-element alignment");
assert.doesNotMatch(appSource, /setPointerCapture|releasePointerCapture|startDragging|DragIntent|continueWindowDragIntent|beginWindowDragIntent/, "custom chrome must not use pointer-capture startDragging loop that freezes render windows");
assert.doesNotMatch(appSource, /titlebar-brand|titlebar-center|side-rail|status-line/, "confirmed baseline UI must not regress to the older chrome layout");

assert.match(styles, /\.window-shell[\s\S]*border:\s*0/, "window shell must not draw an outer border");
assert.match(styles, /\.app-shell[\s\S]*padding:\s*0/, "window shell must not leave a transparent outer gutter");
assert.doesNotMatch(styles, /\.recent-shortcuts|\.recent-drawer-section|\.status-line/, "minimal UI must not keep recent-media or status chrome styles");
assert.match(styles, /playlist-item--active/, "playlist styles must mark the active queue item");
assert.match(styles, /\.history-section/, "styles must include playback history section styling");
assert.match(styles, /\.history-item/, "styles must include playback history item styling");
assert.match(styles, /\.drag-region\s*\{[\s\S]*inset:\s*0/, "drag region must cover the non-control player surface, not just the title strip");
assert.match(styles, /\.resize-region/, "overlay must expose explicit resize hit areas around the border");
assert.match(styles, /\.resize-region--south-east/, "overlay must include corner resize hit areas");
assert.match(styles, /\.transport\s*\{[\s\S]*pointer-events:\s*auto/, "transport controls must remain interactive above the full-surface drag region");
assert.match(styles, /\.playlist-drawer--open\s*\{[\s\S]*pointer-events:\s*auto/, "open playlist drawer must remain interactive above the full-surface drag region");
assert.match(styles, /\.empty-open-logo/, "empty player state must style the OpenPlayer logo");
assert.match(styles, /\.empty-open\s*\{[\s\S]*justify-items:\s*center/, "empty player state must center each logo and text item");
assert.match(styles, /\.stage--chrome-hidden[\s\S]*\.window-controls[\s\S]*opacity:\s*0/, "inactive player chrome must hide window controls");
assert.match(styles, /\.stage--chrome-hidden[\s\S]*\.transport[\s\S]*pointer-events:\s*none/, "inactive player chrome must hide and disable transport controls");
assert.match(styles, /\.context-menu/, "styles must include the custom context menu");
assert.match(styles, /\.settings-dialog/, "styles must include the settings dialog");
assert.match(styles, /\.media-panel/, "styles must include the media options panel for speed and tracks");
assert.match(styles, /\.track-list/, "styles must include track list controls");
assert.match(styles, /\.shortcut-row/, "styles must include shortcut editor rows");
assert.match(styles, /\.seek-control/, "styles must include a seek progress wrapper");
assert.match(styles, /--seek-thumb-size:\s*13px/, "seek control must define the thumb size used for rail alignment");
assert.match(styles, /\.seek-rail[\s\S]*left:\s*calc\(var\(--seek-thumb-size\) \/ 2\)[\s\S]*right:\s*calc\(var\(--seek-thumb-size\) \/ 2\)/, "seek rail must start under the thumb center so frame 0 aligns with the rail origin");
assert.match(styles, /\.seek-rail[\s\S]*top:\s*50%[\s\S]*transform:\s*translateY\(-50%\)/, "seek rail must be vertically centered with the range thumb");
assert.match(styles, /\.seek-progress[\s\S]*width:\s*var\(--progress\)/, "seek progress fill must be driven by the smooth display position");
assert.match(styles, /\.seek-thumb[\s\S]*top:\s*50%[\s\S]*left:\s*calc\(\(var\(--seek-thumb-size\) \/ 2\) \+ \(\(100% - var\(--seek-thumb-size\)\) \* var\(--progress-ratio\)\)\)[\s\S]*transform:\s*translate\(-50%,\s*-50%\)/, "custom seek thumb must share the rail coordinate system and be vertically centered");
assert.match(styles, /\.seek-slider::-webkit-slider-thumb[\s\S]*opacity:\s*0/, "native WebKit range thumb must be hidden so WebView pseudo-element alignment cannot shift the visible thumb");
assert.match(styles, /\.seek-slider::-moz-range-thumb[\s\S]*opacity:\s*0/, "native Firefox range thumb must be hidden so the custom thumb is the only visible marker");
assert.match(styles, /:focus-visible/, "interactive overlays must keep visible keyboard focus states");

assert.match(mpvEmbedSource, /get_property::<bool>\("eof-reached"\)/, "mpv snapshots must read eof-reached to identify true playback end");
assert.match(mpvEmbedSource, /wait_event\(0\.0\)/, "mpv snapshots must drain the event queue without blocking");
assert.match(mpvEmbedSource, /Event::EndFile\(mpv_end_file_reason::Eof\)/, "mpv snapshots must track the real EndFile EOF event");
assert.match(mpvEmbedSource, /get_property::<f64>\("percent-pos"\)/, "mpv snapshots must read percent-pos for near-final-frame detection");
assert.match(mpvEmbedSource, /END_OF_MEDIA_SNAP_TOLERANCE_SECONDS/, "mpv snapshots must define a near-end snap tolerance");
assert.match(mpvEmbedSource, /position:\s*if \(ended \|\| near_end\)[\s\S]*duration/, "mpv snapshots must clamp end-of-file position to duration");
assert.match(mpvEmbedSource, /if ended \{\s*"ended"/, "mpv snapshots must expose ended status at EOF");
assert.match(mpvEmbedSource, /fps:\s*f64/, "mpv snapshots must serialize fps metadata");
assert.match(mpvEmbedSource, /speed:\s*f64/, "mpv snapshots must serialize playback speed");
assert.match(mpvEmbedSource, /subtitle_delay:\s*f64/, "mpv snapshots must serialize subtitle delay");
assert.match(mpvEmbedSource, /pub struct MpvEmbedTrack/, "mpv snapshots must serialize track metadata");
assert.match(mpvEmbedSource, /container-fps/, "mpv snapshots must prefer container-fps for frame-count mode");
assert.match(mpvEmbedSource, /estimated-vf-fps/, "mpv snapshots must fall back to estimated-vf-fps when container fps is unavailable");
assert.match(mpvEmbedSource, /mpv_embed_frame_step/, "mpv embed backend must expose a forward-one-frame command");
assert.match(mpvEmbedSource, /mpv_embed_frame_back_step/, "mpv embed backend must expose a backward-one-frame command");
assert.match(mpvEmbedSource, /mpv_embed_set_speed/, "mpv embed backend must expose playback speed control");
assert.match(mpvEmbedSource, /mpv_embed_set_subtitle_delay/, "mpv embed backend must expose subtitle delay control");
assert.match(mpvEmbedSource, /mpv_embed_select_track/, "mpv embed backend must expose track selection");
assert.match(mpvEmbedSource, /mpv_embed_add_subtitle/, "mpv embed backend must expose external subtitle loading");
assert.match(mpvEmbedSource, /"frame-step"/, "mpv embed backend must call mpv frame-step for forward frame stepping");
assert.match(mpvEmbedSource, /"frame-back-step"/, "mpv embed backend must call mpv frame-back-step for backward frame stepping");
assert.match(mpvEmbedSource, /track-list\/count/, "mpv embed backend must read mpv track-list metadata");
assert.match(mpvEmbedSource, /sub-add/, "mpv embed backend must use mpv sub-add for external subtitles");
assert.match(mpvEmbedSource, /discover_sidecar_subtitles/, "mpv embed backend must discover same-folder sidecar subtitles when opening media");
assert.match(mpvEmbedSource, /"sub-delay"/, "mpv embed backend must control mpv sub-delay for subtitle sync");
assert.match(mpvEmbedSource, /#\[cfg\(windows\)\][\s\S]*use windows_sys::Win32/, "mpv native video host must keep Win32 imports behind a Windows platform gate");
assert.match(mpvEmbedSource, /#\[cfg\(not\(windows\)\)\][\s\S]*mpv embed playback currently supports Windows HWND hosts only/, "mpv native video host must return an explicit unsupported-platform error outside Windows");
assert.match(mpvEmbedSource, /fn wid\(&self\) -> i64/, "mpv native video host must expose a platform-owned mpv wid boundary");
assert.match(mpvEmbedSource, /mpv_embed_snapshot[\s\S]*player\.snapshot\(0,\s*"playing"\)/, "periodic mpv snapshots must preserve playing status so smooth progress, frame labels, and Space pause keep working");
assert.match(mpvEmbedSource, /input-default-bindings"[\s\S]*false/, "embedded mpv must not keep its own default keyboard bindings when the video background has focus");
assert.match(mpvEmbedSource, /input-vo-keyboard"[\s\S]*false/, "embedded mpv video output must not consume OpenPlayer shortcuts");
assert.match(mpvEmbedSource, /force_paused_until/, "frame stepping must guard snapshots against mpv's transient unpaused frame-step state");
assert.match(mpvEmbedSource, /FRAME_STEP_PAUSE_GUARD/, "frame stepping must define a bounded paused-state guard");
assert.match(mpvEmbedSource, /settle_frame_step_pause/, "frame stepping must wait briefly for mpv to return to paused state");
assert.match(mpvEmbedSource, /raw_paused\s*\|\|\s*pause_guard_active/, "snapshots must report paused while the frame-step guard is active");

assert.doesNotMatch(tauriLibSource, /mod playback;|mod storage|DesktopPlaybackState|DesktopStorageState|playback_command|storage_|openplayer_core|openplayer_shared/, "desktop backend must not restore removed playback or storage plumbing");
assert.match(tauriLibSource, /window_minimize/, "desktop backend must keep minimize command");
assert.match(tauriLibSource, /window_toggle_maximize/, "desktop backend must keep maximize command");
assert.match(tauriLibSource, /window_toggle_fullscreen[\s\S]*main_window\(&app\)\?[\s\S]*set_fullscreen/, "desktop backend must toggle fullscreen on the main video window");
assert.match(tauriLibSource, /struct WindowPlacement/, "desktop backend must record window placement before entering fullscreen");
assert.match(tauriLibSource, /restore_window_after_fullscreen/, "desktop backend must restore the recorded placement after leaving fullscreen");
assert.match(tauriLibSource, /fn focus_overlay_window/, "desktop backend must provide a shared way to focus the overlay controls window");
assert.match(tauriLibSource, /fn window_focus_overlay/, "desktop backend must expose an overlay focus command for frontend shortcut recovery");
assert.match(tauriLibSource, /fn window_update_shortcuts/, "desktop backend must accept current custom shortcuts for native dispatch");
assert.match(tauriLibSource, /mod playback_store;/, "desktop backend must include the playback history store module");
assert.match(tauriLibSource, /PlaybackStoreState/, "desktop backend must manage playback history store state");
assert.match(tauriLibSource, /history_list[\s\S]*history_remember[\s\S]*history_resume_position/, "desktop runtime must register playback history commands");
assert.match(tauriCargoToml, /redb/, "desktop backend must depend on redb for high-performance playback history storage");
assert.match(playbackStoreSource, /redb::Database|use redb::\{Database/, "playback history store must use redb");
assert.match(playbackStoreSource, /HISTORY_BY_PATH/, "playback history store must index entries by path");
assert.match(playbackStoreSource, /HISTORY_BY_UPDATED/, "playback history store must index entries by recency");
assert.match(playbackStoreSource, /resume_position_for_entry/, "playback history store must compute resume positions by ratio");
assert.match(playbackStoreSource, /RESUME_END_PROGRESS_RATIO/, "playback history store must define a relative end threshold");
assert.match(tauriLibSource, /fn window_set_shortcuts_enabled/, "desktop backend must allow frontend modal states to suspend native shortcut dispatch");
assert.match(tauriLibSource, /fn install_native_shortcut_hook/, "desktop backend must install a native shortcut bridge for focus-independent shortcuts on Windows");
assert.match(tauriLibSource, /SetWindowsHookExW/, "Windows shortcut bridge must use a low-level keyboard hook while the app is focused");
assert.match(tauriLibSource, /GetModuleHandleW/, "Windows shortcut hook must pass the current module handle instead of silently failing with a null module");
assert.match(tauriLibSource, /GetForegroundWindow/, "Windows shortcut bridge must only dispatch shortcuts when OpenPlayer is the foreground app");
assert.match(tauriLibSource, /openplayer-native-shortcut/, "native shortcut bridge must emit actions to the overlay frontend");
assert.match(tauriLibSource, /sync_overlay_to_main[\s\S]*focus_overlay_window\(app\)/, "overlay sync must return keyboard focus to the controls window");
assert.match(tauriLibSource, /WindowEvent::Focused\(true\)[\s\S]*focus_overlay_window\(&app_handle\)/, "clicking the video/main window must return keyboard focus to the overlay shortcut handler");
assert.match(tauriLibSource, /fn schedule_overlay_sync_to_main/, "desktop backend must schedule overlay sync after asynchronous fullscreen transitions");
assert.match(windowToggleFullscreenSource, /schedule_overlay_sync_to_main\(&app\)/, "fullscreen toggling must defer overlay sync until the main window has applied fullscreen bounds");
assert.doesNotMatch(windowToggleFullscreenSource, /sync_overlay_to_main\(&app\)/, "fullscreen toggling must not immediately sync the overlay using stale fullscreen transition bounds");
assert.match(mpvEmbedRunSource, /mpv_embed_frame_step[\s\S]*mpv_embed_frame_back_step[\s\S]*mpv_embed_set_speed[\s\S]*mpv_embed_set_subtitle_delay[\s\S]*mpv_embed_select_track[\s\S]*mpv_embed_add_subtitle/, "desktop runtime must register frame, speed, subtitle delay, track, and subtitle mpv commands");
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

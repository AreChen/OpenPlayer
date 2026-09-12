import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");
for (const runtime of ["embedded", "fallback"]) {
  const bootstrap = await source(`src-tauri/src/bootstrap/${runtime}.rs`);
  for (const command of ["list", "start", "call", "stop", "validate_video_plan", "video_attach", "video_detach", "video_status", "video_refresh_paused"]) {
    assert(bootstrap.includes(`crate::plugin_native::plugin_native_${command}`), `${runtime} must register native ${command}`);
  }
  assert.match(bootstrap, /RunEvent::Exit[\s\S]*plugin_native::shutdown/, `${runtime} must clean up native modules on exit`);
  assert(!bootstrap.includes("native_filter_smoke"), "the developer frame probe must not be exposed through IPC");
}

const mpvModules = await source("src-tauri/src/mpv_embed/mod.rs");
assert.match(mpvModules, /#\[cfg\(feature = "window-smoke"\)\]\s*pub\(crate\) mod native_filter_smoke;/,
  "the experimental frame adapter must stay behind the window-smoke feature");
const smokeAdapter = await source("src-tauri/src/mpv_embed/native_filter_smoke.rs");
assert(!smokeAdapter.includes("#[tauri::command]"), "smoke helpers are not plugin APIs");
const video = await source("src-tauri/src/plugin_native/video.rs");
assert.match(video, /start the installed native module first/);
assert.match(video, /session\.launch\.package_root/, "video runtimes must resolve from the authorized installed session");
assert(!video.includes("OPENPLAYER_SMOKE"), "public attachment cannot use developer environment overrides");
assert.match(video, /frame_options\(options\)/, "host conversion options must be separated from plugin controls");
const filter = await source("src-tauri/src/mpv_embed/native_video_filter.rs");
assert.match(filter, /self.normalize\s*\{\s*"yuv420p10"\s*\}[\s\S]*native_video_color::sdr_filter/, "download hardware frames without discarding DV precision before shared color conversion");
assert.match(filter, /-rate:lavfi=\[fps=fps=[\s\S]*native_video_color::sdr_filter/, "limit rate before expensive color conversion and native processing");
const color = await source("src-tauri/src/mpv_embed/native_video_color.rs");
assert.match(color, /libplacebo=apply_dolbyvision=true:colorspace=bt709:color_primaries=bt709:color_trc=bt709:range=tv:format=yuv420p/, "both adapters must retain the fixed DV-aware SDR conversion graph");
assert.match(color, /-download:format=fmt=yuv420p10,[\s\S]*sdr_filter/, "presentation conversion must preserve input precision");
const presentation = await source("src-tauri/src/mpv_embed/native_presentation/mod.rs");
assert.match(presentation, /conversion_requested\(options.remove\("inputConversion"\)\)/, "presentation consumes the host-owned conversion option");
assert.match(presentation, /transaction::switch[\s\S]*validate_media\(&player.mpv, false\)\?[\s\S]*\.store\(true, Ordering::Release\)/, "validate converted pixels before publishing active presentation");
assert.match(presentation, /fn restore[\s\S]*conversion.remove\(mpv\)\?[\s\S]*transaction::switch/, "restore original color/output after presentation");
assert.match(filter, /native-input-/, "conversion filters require independent owned labels and cleanup");
assert.match(await source("src-tauri/src/mpv_embed/commands/lifecycle.rs"), /media_change_guard[\s\S]*stop_existing_player_for_replacement/);

const worker = await source("src/app/pluginRuntime/workerSource/apiSections.ts");
const view = await source("src/app/pluginRuntime/viewDocument.ts");
assert(worker.includes("pluginWorkerNativeApiSource()"));
assert(view.includes("pluginWorkerNativeApiSource()"), "workers and custom views must reuse the same native API source");
assert(view.includes("pluginWorkerFilesystemApiSource()"), "custom views must reuse the permissioned filesystem picker bridge");

const commands = await source("src-tauri/src/plugin_native/commands.rs");
assert.match(commands, /async fn plugin_native_stop[\s\S]*spawn_blocking/, "native stop cannot wait for process cleanup on the window thread");
assert.match(commands, /verify_executable[\s\S]*generation != generation[\s\S]*Session::spawn/, "verify lifecycle snapshot and executable before starting native code");
assert.doesNotMatch(commands, /blocking_show|app\.dialog\(/, "installed plugins must not prompt again when starting declared native modules");

const tree = await source("src-tauri/src/plugin_native/process_tree.rs");
assert(tree.includes("CREATE_SUSPENDED"));
assert(tree.includes("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE"));
assert.match(tree, /AssignProcessToJobObject[\s\S]*resume_initial_thread/, "Windows workers must join the job before executing");
const plan = await source("src-tauri/src/plugin_native/video_plan.rs");
assert.match(plan, /executable: false/, "a validated plan must not claim live playback execution");
const packageSource = await source("src-tauri/src/appearance_store/package.rs");
assert.match(packageSource, /fn validate_native_package[\s\S]*verify_executable/, "installation must verify packaged native executables");
const lifecycle = await source("src-tauri/src/appearance_store/commands.rs");
for (const command of ["appearance_import_plugin_package", "appearance_import_plugin_directory", "appearance_uninstall_plugin", "appearance_set_plugin_enabled"]) {
  assert(lifecycle.includes(`async fn ${command}`), `${command} must not wait for native cleanup on the window thread`);
}

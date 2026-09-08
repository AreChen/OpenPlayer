import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");
for (const runtime of ["embedded", "fallback"]) {
  const bootstrap = await source(`src-tauri/src/bootstrap/${runtime}.rs`);
  for (const command of ["list", "start", "call", "stop", "validate_video_plan"]) {
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

const worker = await source("src/app/pluginRuntime/workerSource/apiSections.ts");
const view = await source("src/app/pluginRuntime/viewDocument.ts");
assert(worker.includes("pluginWorkerNativeApiSource()"));
assert(view.includes("pluginWorkerNativeApiSource()"), "workers and custom views must reuse the same native API source");

const commands = await source("src-tauri/src/plugin_native/commands.rs");
assert.match(commands, /async fn plugin_native_stop[\s\S]*spawn_blocking/, "native stop cannot wait for process cleanup on the window thread");
assert.match(commands, /verify_executable[\s\S]*generation != generation[\s\S]*Session::spawn/, "verify consent snapshot and executable before starting native code");
assert.match(commands, /blocking_show/, "native launch requires a host-owned confirmation");

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

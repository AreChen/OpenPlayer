import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");

const [mainWindow] = config.app.windows;

assert.equal(mainWindow.url, "index.html", "packaged exe must load the bundled app entry");
assert.equal(mainWindow.decorations, false, "window must disable native decorations for custom titlebar");
assert.equal(mainWindow.transparent, true, "window must be transparent so CSS rounded corners are visible");
assert.equal(mainWindow.shadow, true, "window should keep native shadow when available");
assert.match(config.build.devUrl, /23142$/, "Tauri dev URL must use the non-reserved Windows port");
assert.match(packageJson.scripts.dev, /23142$/, "Vite dev script must use the non-reserved Windows port");
assert.match(packageJson.scripts.preview, /23142$/, "Vite preview script must use the non-reserved Windows port");
assert.match(appSource, /data-tauri-drag-region/, "custom titlebar must expose a Tauri drag region");
assert.match(appSource, /window_minimize/, "custom titlebar must wire minimize command");
assert.match(appSource, /window_toggle_maximize/, "custom titlebar must wire maximize command");
assert.match(appSource, /window_close/, "custom titlebar must wire close command");
assert.match(appSource, /<video/, "player shell must include an actual media element");
assert.match(appSource, /type="file"/, "player shell must expose local file open support");
assert.match(appSource, /onDrop=/, "player shell must support drag-and-drop media loading");
assert.match(appSource, /togglePlayback/, "player shell must wire play and pause behavior");
assert.match(appSource, /seekTo/, "player shell must wire timeline seeking behavior");
assert.match(appSource, /setVolume/, "player shell must wire volume behavior");
assert.match(styles, /border-radius:\s*var\(--window-radius\)/, "window shell must own rounded corners");
assert.match(styles, /background:\s*transparent/, "document background must allow transparent window corners");

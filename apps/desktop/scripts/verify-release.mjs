import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";

const root = new URL("../../../", import.meta.url);

function rootFile(path) {
  return new URL(path, root);
}

function fail(message) {
  throw new Error(`release metadata check failed: ${message}`);
}

async function readText(path) {
  return readFile(rootFile(path), "utf8");
}

async function readJson(path) {
  return JSON.parse(await readText(path));
}

function matchVersion(source, pattern, label) {
  const match = pattern.exec(source);
  if (!match) {
    fail(`missing ${label} version`);
  }
  return match[1];
}

function cargoLockPackageVersion(source, packageName) {
  const packageBlocks = source.split(/\r?\n\[\[package\]\]\r?\n/);
  for (const block of packageBlocks) {
    if (new RegExp(`name = "${packageName}"`).test(block)) {
      return matchVersion(block, /^version = "([^"]+)"$/m, `Cargo.lock package ${packageName}`);
    }
  }
  fail(`missing Cargo.lock package ${packageName}`);
}

const workspaceToml = await readText("Cargo.toml");
const cargoLock = await readText("Cargo.lock");
const packageJson = await readJson("apps/desktop/package.json");
const packageLock = await readJson("apps/desktop/package-lock.json");
const tauriConfig = await readJson("apps/desktop/src-tauri/tauri.conf.json");
const pluginRuntimeConstants = await readText("apps/desktop/src/app/pluginRuntime/constants.ts");
const releaseWorkflow = await readText(".github/workflows/release.yml");

const versions = new Map([
  ["Cargo workspace", matchVersion(workspaceToml, /\[workspace\.package\][\s\S]*?^version = "([^"]+)"$/m, "Cargo workspace")],
  ["Cargo.lock openplayer-desktop", cargoLockPackageVersion(cargoLock, "openplayer-desktop")],
  ["package.json", packageJson.version],
  ["package-lock.json", packageLock.version],
  ["package-lock root package", packageLock.packages?.[""]?.version],
  ["tauri.conf.json", tauriConfig.version],
  ["plugin runtime SDK", matchVersion(pluginRuntimeConstants, /PLUGIN_SDK_VERSION = "([^"]+)"/, "plugin runtime SDK")],
  ["plugin runtime host", matchVersion(pluginRuntimeConstants, /OPENPLAYER_HOST_VERSION = "([^"]+)"/, "plugin runtime host")],
]);

const uniqueVersions = new Set(versions.values());
if (uniqueVersions.size !== 1) {
  const details = [...versions.entries()].map(([label, version]) => `${label}: ${version}`).join("\n");
  fail(`version mismatch\n${details}`);
}

const [version] = uniqueVersions;
if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  fail(`version is not valid semver: ${version}`);
}

const expectedTag = `v${version}`;
const cliTag = process.argv.find((arg) => arg.startsWith("--tag="))?.slice("--tag=".length);
const releaseTag = cliTag || process.env.RELEASE_TAG || (process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : "");
if (releaseTag && releaseTag !== expectedTag) {
  fail(`tag ${releaseTag} does not match package version ${expectedTag}`);
}

const releaseNotesPath = `docs/releases/${expectedTag}.md`;
if (!existsSync(rootFile(releaseNotesPath))) {
  fail(`missing release notes: ${releaseNotesPath}`);
}

if (!/ln -s \/Applications "\$dmg_root\/Applications"/.test(releaseWorkflow)) {
  fail("macOS DMG workflow must add an Applications symlink for drag-and-drop installation");
}

if (!/hdiutil create[\s\S]*-srcfolder "\$dmg_root"/.test(releaseWorkflow)) {
  fail("macOS DMG workflow must build from the staged DMG folder, not directly from OpenPlayer.app");
}

console.log(`release metadata ok: ${version}`);

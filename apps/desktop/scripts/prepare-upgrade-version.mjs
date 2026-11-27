import { readFile, writeFile } from "node:fs/promises";

const [tag] = process.argv.slice(2);
if (!/^v2\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag ?? "")) {
  throw new Error("Expected a stable OpenPlayer 2.x fixture tag");
}
const source = tag.slice(1);
const [major, minor, patch] = source.split(".");
const target = `${major}.${minor}.${BigInt(patch) + 1n}`;
const root = new URL("../../../", import.meta.url);

async function update(path, rewrite) {
  const file = new URL(path, root);
  const previous = await readFile(file, "utf8");
  const next = rewrite(previous);
  if (previous === next) throw new Error(`Fixture version did not change in ${path}`);
  await writeFile(file, next);
}

function versionLine(text) {
  const previous = `version = "${source}"`;
  if (!text.includes(previous)) throw new Error(`Expected fixture source version ${source}`);
  return text.replace(previous, `version = "${target}"`);
}

await update("Cargo.toml", versionLine);
await update("Cargo.lock", (text) => {
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const separator = `${newline}[[package]]${newline}`;
  return text.split(separator)
    .map((block) => block.startsWith(`name = "openplayer-desktop"${newline}`) ? versionLine(block) : block)
    .join(separator);
});

async function updateJson(path, lock = false) {
  await update(path, (text) => {
    const metadata = JSON.parse(text);
    if (metadata.version !== source || (lock && metadata.packages[""].version !== source)) {
      throw new Error(`Unexpected version in ${path}`);
    }
    metadata.version = target;
    if (lock) metadata.packages[""].version = target;
    return `${JSON.stringify(metadata, null, 2)}\n`;
  });
}

await updateJson("apps/desktop/package.json");
await updateJson("apps/desktop/package-lock.json", true);
await updateJson("apps/desktop/src-tauri/tauri.conf.json");
await update("apps/desktop/src/app/pluginRuntime/constants.ts", (text) =>
  text.replace(`OPENPLAYER_HOST_VERSION = "${source}"`, `OPENPLAYER_HOST_VERSION = "${target}"`));
await writeFile(new URL(`docs/releases/v${target}.md`, root), `# OpenPlayer ${target}\n\nIsolated update fixture for ${tag}.\n`, { flag: "wx" });
console.log(`v${target}`);

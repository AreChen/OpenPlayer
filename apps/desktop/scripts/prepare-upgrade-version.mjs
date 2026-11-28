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

await writeFile(new URL(`docs/releases/v${target}.md`, root), `# OpenPlayer ${target}\n\nIsolated update fixture for ${tag}.\n`, { flag: "wx" });
console.log(`v${target}`);

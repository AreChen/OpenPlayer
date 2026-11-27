import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { copyFile, mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

const [operation, tag, ...args] = process.argv.slice(2);
if (!/^v\d+\.\d+\.\d+$/.test(tag ?? "")) {
  throw new Error(`Expected a stable release tag, found ${tag}`);
}

const version = tag.slice(1);
const stem = `OpenPlayer_${version}`;
const platforms = {
  "windows-x64": [["nsis", `${stem}_x64-setup.exe`]],
  "linux-x64": [["deb", `${stem}_amd64.deb`], ["appimage", `${stem}_amd64.AppImage`]],
  "macos-arm64": [["dmg", `${stem}_arm64.dmg`]],
  "macos-x64": [["dmg", `${stem}_x64.dmg`]],
};

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
  }
  return hash.digest("hex");
}

function checksumLine(hash, name) {
  return `${hash}  ${name}\n`;
}

async function stage(platform, destination, distribution) {
  const packages = platforms[platform];
  if (!packages || !validDistribution(platform, distribution)) {
    throw new Error(`Unexpected platform or distribution mode: ${platform} ${distribution}`);
  }

  const commit = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  const bundle = join(process.env.CARGO_TARGET_DIR || "target", "release", "bundle");
  await mkdir(destination, { recursive: true });

  const assets = [];
  for (const [directory, name] of packages) {
    const source = join(bundle, directory, name);
    if ((await stat(source)).size === 0) {
      throw new Error(`Cannot stage an empty package: ${name}`);
    }
    const packagePath = join(destination, name);
    await copyFile(source, packagePath);
    const hash = await sha256(packagePath);
    await writeFile(`${packagePath}.sha256`, checksumLine(hash, name));
    assets.push({ name, sha256: hash });
  }
  const manifest = { tag, commit, platform, distribution, assets };
  await writeFile(join(destination, "build.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}

function validDistribution(platform, mode) {
  return platform.startsWith("macos-") ? ["ad-hoc", "notarized"].includes(mode) : mode === "native";
}

async function assemble(commit, artifactRoot, destination) {
  if (!/^[a-f0-9]{40}$/.test(commit)) {
    throw new Error(`Expected an exact source commit, found ${commit}`);
  }

  const builds = [];
  await mkdir(destination, { recursive: true });
  for (const [platform, packages] of Object.entries(platforms)) {
    const artifact = join(artifactRoot, `openplayer-${platform}`);
    const build = JSON.parse(await readFile(join(artifact, "build.json"), "utf8"));
    if (build.tag !== tag || build.commit !== commit || build.platform !== platform ||
        !validDistribution(platform, build.distribution) || build.assets?.length !== packages.length) {
      throw new Error(`Inconsistent build manifest for ${platform}`);
    }

    const expectedFiles = ["build.json", ...packages.flatMap(([, name]) => [name, `${name}.sha256`])];
    if ((await readdir(artifact)).sort().join("\n") !== expectedFiles.sort().join("\n")) {
      throw new Error(`Unexpected package files for ${platform}`);
    }

    for (const [, name] of packages) {
      const source = join(artifact, name);
      const hash = await sha256(source);
      if (build.assets.find((asset) => asset.name === name)?.sha256 !== hash ||
          await readFile(`${source}.sha256`, "utf8") !== checksumLine(hash, name)) {
        throw new Error(`SHA-256 verification failed for ${name}`);
      }
      await copyFile(source, join(destination, name));
      await copyFile(`${source}.sha256`, join(destination, `${name}.sha256`));
    }
    builds.push(build);
  }
  const manifest = { tag, commit, builds };
  await writeFile(join(destination, "release-assets.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}

if (operation === "stage" && args.length === 3) {
  await stage(...args);
} else if (operation === "assemble" && args.length === 3) {
  await assemble(...args);
} else {
  throw new Error("Use stage TAG PLATFORM OUTPUT DISTRIBUTION or assemble TAG COMMIT INPUT OUTPUT");
}

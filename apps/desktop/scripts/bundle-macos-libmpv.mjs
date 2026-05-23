import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readlinkSync,
  readdirSync,
  rmSync,
  statSync,
  symlinkSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("../../../", import.meta.url)));
const defaultApp = join(root, "target/release/bundle/macos/OpenPlayer.app");
const appBundle = resolve(process.argv[2] ?? defaultApp);
const frameworksDir = join(appBundle, "Contents/Frameworks");
const executableDir = join(appBundle, "Contents/MacOS");

function fail(message) {
  throw new Error(`macOS libmpv bundling failed: ${message}`);
}

function command(name, args) {
  return execFileSync(name, args, { encoding: "utf8" });
}

function appExecutable() {
  if (!existsSync(executableDir)) {
    fail(`missing executable directory: ${executableDir}`);
  }

  const candidates = readdirSync(executableDir)
    .map((entry) => join(executableDir, entry))
    .filter((path) => !path.endsWith(".dSYM"));
  if (candidates.length !== 1) {
    fail(`expected one app executable in ${executableDir}, found ${candidates.length}`);
  }

  return candidates[0];
}

function dylibDependencies(binary) {
  let output = "";
  try {
    output = command("otool", ["-L", binary]);
  } catch {
    return [];
  }

  return output
    .split(/\r?\n/)
    .slice(1)
    .map((line) => line.trim().split(/\s+/)[0])
    .filter(Boolean);
}

function inspectableBinaries(root) {
  const stat = statSync(root);
  if (stat.isFile()) {
    return isInspectableBinary(root, stat) ? [root] : [];
  }

  const binaries = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const child = join(root, entry.name);
    if (entry.isDirectory()) {
      binaries.push(...inspectableBinaries(child));
    } else if (entry.isFile()) {
      const childStat = statSync(child);
      if (isInspectableBinary(child, childStat)) {
        binaries.push(child);
      }
    }
  }

  return binaries;
}

function isInspectableBinary(path, stat) {
  return path.endsWith(".dylib") || path.endsWith(".so") || Boolean(stat.mode & 0o111);
}

function isInstallNameTarget(path) {
  return path.endsWith(".dylib") || path.includes(".framework/");
}

function forceSymlink(target, linkPath) {
  rmSync(linkPath, { force: true, recursive: true });
  symlinkSync(target, linkPath);
}

function removeCodeSignatureDirectories(root) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const child = join(root, entry.name);
    if (entry.name === "_CodeSignature") {
      rmSync(child, { force: true, recursive: true });
    } else if (entry.isDirectory()) {
      removeCodeSignatureDirectories(child);
    }
  }
}

function rewriteFrameworkSymlinks(root, sourceRoot, targetRoot) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const child = join(root, entry.name);
    const childStat = lstatSync(child);
    if (childStat.isSymbolicLink()) {
      const target = readlinkSync(child);
      if (target.startsWith(sourceRoot)) {
        const localTarget = join(targetRoot, target.slice(sourceRoot.length + 1));
        forceSymlink(relative(dirname(child), localTarget), child);
      }
    } else if (entry.isDirectory()) {
      rewriteFrameworkSymlinks(child, sourceRoot, targetRoot);
    }
  }
}

function normalizeFrameworkBundle(sourceRoot, targetRoot) {
  rewriteFrameworkSymlinks(targetRoot, sourceRoot, targetRoot);
  removeCodeSignatureDirectories(targetRoot);

  const versionsDir = join(targetRoot, "Versions");
  if (!existsSync(versionsDir)) {
    return;
  }

  const versions = readdirSync(versionsDir)
    .filter((entry) => entry !== "Current")
    .filter((entry) => lstatSync(join(versionsDir, entry)).isDirectory())
    .sort();
  const version = versions.at(-1);
  if (!version) {
    return;
  }

  const versionRoot = join(versionsDir, version);
  const frameworkBinary = basename(targetRoot, ".framework");
  if (existsSync(join(versionRoot, frameworkBinary))) {
    forceSymlink(version, join(versionsDir, "Current"));
    forceSymlink(`Versions/Current/${frameworkBinary}`, join(targetRoot, frameworkBinary));
  }
  if (existsSync(join(versionRoot, "Headers"))) {
    forceSymlink("Versions/Current/Headers", join(targetRoot, "Headers"));
  }
  if (existsSync(join(versionRoot, "Resources"))) {
    forceSymlink("Versions/Current/Resources", join(targetRoot, "Resources"));
  }

  if (frameworkBinary === "Python") {
    const pythonHeaders = join(versionRoot, "include", `python${version}`);
    if (existsSync(pythonHeaders)) {
      forceSymlink(`include/python${version}`, join(versionRoot, "Headers"));
    }

    const sitePackages = join(versionRoot, "lib", `python${version}`, "site-packages");
    if (existsSync(sitePackages) && lstatSync(sitePackages).isSymbolicLink()) {
      rmSync(sitePackages, { force: true });
      mkdirSync(sitePackages, { recursive: true });
    }
  }
}

function hasBundledLibmpv() {
  return readdirSync(frameworksDir).some(
    (entry) => entry.startsWith("libmpv") && entry.endsWith(".dylib"),
  );
}

function shouldBundleDependency(path) {
  return (
    path.startsWith("/opt/homebrew/") || path.startsWith("/usr/local/") || path.includes("/Cellar/")
  ) && (path.endsWith(".dylib") || path.includes(".framework/"));
}

function bundledDependency(path) {
  const frameworkMarker = ".framework/";
  const frameworkIndex = path.indexOf(frameworkMarker);
  if (frameworkIndex !== -1) {
    const frameworkRoot = path.slice(0, frameworkIndex + ".framework".length);
    const frameworkName = basename(frameworkRoot);
    const relativeBinary = path.slice(frameworkRoot.length + 1);
    const copyTarget = join(frameworksDir, frameworkName);
    return {
      copySource: frameworkRoot,
      copyTarget,
      targetBinary: join(copyTarget, relativeBinary),
      reference: `@executable_path/../Frameworks/${frameworkName}/${relativeBinary}`,
    };
  }

  const name = basename(path);
  const target = join(frameworksDir, name);
  return {
    copySource: path,
    copyTarget: target,
    targetBinary: target,
    reference: `@executable_path/../Frameworks/${name}`,
  };
}

function queueBinary(path, queue, queuedTargets) {
  if (queuedTargets.has(path)) {
    return;
  }

  queuedTargets.add(path);
  queue.push(path);
}

function copyDependency(path, queue, bundled, queuedTargets) {
  const dependency = bundledDependency(path);
  if (!existsSync(dependency.copyTarget)) {
    if (dependency.copySource.endsWith(".framework")) {
      cpSync(dependency.copySource, dependency.copyTarget, {
        recursive: true,
        preserveTimestamps: true,
      });
    } else {
      copyFileSync(dependency.copySource, dependency.copyTarget);
    }
  }

  if (dependency.copySource.endsWith(".framework")) {
    normalizeFrameworkBundle(dependency.copySource, dependency.copyTarget);
  }

  chmodSync(dependency.targetBinary, 0o755);

  if (!bundled.has(path)) {
    bundled.set(path, dependency);
  }

  for (const binary of inspectableBinaries(dependency.copyTarget)) {
    queueBinary(binary, queue, queuedTargets);
  }

  return dependency;
}

function rewriteDependencyReferences(binary, replacements) {
  const dependencies = new Set(dylibDependencies(binary));
  for (const [original, dependency] of replacements) {
    if (!dependencies.has(original)) {
      continue;
    }

    command("install_name_tool", ["-change", original, dependency.reference, binary]);
  }
}

if (!existsSync(appBundle)) {
  fail(`missing app bundle: ${appBundle}`);
}

mkdirSync(frameworksDir, { recursive: true });

const executable = appExecutable();
const bundled = new Map();
const queue = [];
const queuedTargets = new Set();

queueBinary(executable, queue, queuedTargets);
for (const binary of inspectableBinaries(frameworksDir)) {
  queueBinary(binary, queue, queuedTargets);
}

for (const dependency of dylibDependencies(executable).filter(shouldBundleDependency)) {
  copyDependency(dependency, queue, bundled, queuedTargets);
}

for (let index = 0; index < queue.length; index += 1) {
  const binary = queue[index];
  const dependencies = dylibDependencies(binary).filter(shouldBundleDependency);

  for (const dependency of dependencies) {
    copyDependency(dependency, queue, bundled, queuedTargets);
  }
}

if (![...bundled.keys()].some((path) => basename(path).startsWith("libmpv")) && !hasBundledLibmpv()) {
  fail(`no Homebrew libmpv dependency found in ${executable}`);
}

for (const [source, copied] of bundled) {
  if (isInstallNameTarget(copied.targetBinary)) {
    command("install_name_tool", ["-id", copied.reference, copied.targetBinary]);
  }

  console.log(`bundled ${source} -> ${copied.targetBinary}`);
}

for (const binary of queuedTargets) {
  rewriteDependencyReferences(binary, bundled);
}

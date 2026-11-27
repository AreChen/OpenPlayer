import {
  chmodSync,
  closeSync,
  copyFileSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  openSync,
  readlinkSync,
  readSync,
  readdirSync,
  realpathSync,
  rmSync,
  statSync,
  symlinkSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("../../../", import.meta.url)));
const defaultApp = join(root, "target/release/bundle/macos/OpenPlayer.app");
const args = process.argv.slice(2);
const verifyOnly = args[0] === "--verify";
const requestedApp = resolve(args[verifyOnly ? 1 : 0] ?? defaultApp);
if (!existsSync(requestedApp)) fail(`missing app bundle: ${requestedApp}`);
const appBundle = realpathSync(requestedApp);
const frameworksDir = join(appBundle, "Contents/Frameworks");
const executableDir = join(appBundle, "Contents/MacOS");
const architecture = process.arch === "x64" ? "x86_64" : process.arch;
const loadCommands = new Set([
  "LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LOAD_UPWARD_DYLIB", "LC_LAZY_LOAD_DYLIB",
]);

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

function machoDescription(binary) {
  const output = command("otool", ["-arch", architecture, "-l", binary]);
  const description = { installName: null, dependencies: [], runpaths: [] };
  for (const block of output.split(/^[ \t]*Load command [0-9]+[ \t]*$/m)) {
    const type = block.match(/^[ \t]*cmd[ \t]+(LC_\w+)[ \t]*$/m)?.[1];
    const path = block.match(/^[ \t]*(?:name|path)[ \t]+(.+) \(offset [0-9]+\)[ \t]*$/m)?.[1];
    if (!path) continue;
    if (type === "LC_RPATH") description.runpaths.push(path);
    if (type === "LC_ID_DYLIB") description.installName = path;
    if (loadCommands.has(type)) description.dependencies.push(path);
  }
  return description;
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
  if (!path.endsWith(".dylib") && !path.endsWith(".so") && !(stat.mode & 0o111)) return false;
  const magic = Buffer.alloc(4);
  const file = openSync(path, "r");
  try {
    return readSync(file, magic, 0, magic.length, 0) === magic.length &&
      /^(feedface|feedfacf|cefaedfe|cffaedfe|cafebabe|bebafeca|cafebabf|bfbafeca)$/.test(magic.toString("hex"));
  } finally {
    closeSync(file);
  }
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

function systemLibrary(path) {
  if (!isAbsolute(path)) return false;
  const absolute = resolve(path);
  return absolute.startsWith("/usr/lib/") || absolute.startsWith("/System/Library/");
}

function localPaths(reference, source, runpaths) {
  if (isAbsolute(reference)) return [resolve(reference)];
  const [anchor, ...segments] = reference.split("/");
  if (anchor === "@loader_path") {
    return [...new Set([dirname(source), dirname(realpathSync(source))])].map((dir) => resolve(dir, ...segments));
  }
  if (anchor === "@executable_path") return [resolve(executableDir, ...segments)];
  if (anchor === "@rpath") return runpaths.map((dir) => resolve(dir, ...segments));
  if (anchor.startsWith("@")) return [];
  return [resolve(dirname(source), reference)];
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
    };
  }

  const name = basename(path);
  const target = join(frameworksDir, name);
  return {
    copySource: path,
    copyTarget: target,
    targetBinary: target,
  };
}

const executable = appExecutable();

function verifyBundle() {
  const binaries = [executable, ...inspectableBinaries(frameworksDir)];
  const bundled = new Set(binaries.map((binary) => realpathSync(binary)));
  const app = machoDescription(executable);
  if (!app.dependencies.some((path) => /^@loader_path\/\.\.\/Frameworks\/libmpv[^/]*\.dylib$/.test(path))) {
    fail(`the app executable does not load libmpv from ${frameworksDir}`);
  }

  for (const binary of binaries) {
    const description = binary === executable ? app : machoDescription(binary);
    for (const reference of description.dependencies) {
      if (systemLibrary(reference)) continue;
      const local = reference.startsWith("@loader_path/") && localPaths(reference, binary, []).some(
        (path) => existsSync(path) && bundled.has(realpathSync(path)),
      );
      if (!local) fail(`unbundled dependency ${reference} in ${binary}`);
    }
  }
  console.log(`Verified bundled load commands in ${binaries.length} Mach-O binaries`);
}

function bundleDependencies() {
  mkdirSync(frameworksDir, { recursive: true });
  const targetsBySource = new Map();
  const copies = new Map();
  const edits = new Map();
  const extraImages = inspectableBinaries(frameworksDir).map((path) => ({ source: path, target: path, runpaths: [] }));

  function copyImage(source, runpaths) {
    const canonical = realpathSync(source);
    if (canonical.startsWith(`${appBundle}/`)) return canonical;

    const { copySource, copyTarget, targetBinary } = bundledDependency(source);
    const previous = copies.get(copyTarget);
    if (previous && previous !== realpathSync(copySource)) fail(`conflicting dependency at ${copyTarget}`);
    if (!previous) {
      if (existsSync(copyTarget)) fail(`dependency would overwrite ${copyTarget}`);
      copies.set(copyTarget, realpathSync(copySource));
      if (copySource.endsWith(".framework")) {
        cpSync(copySource, copyTarget, { recursive: true, preserveTimestamps: true });
        normalizeFrameworkBundle(copySource, copyTarget);
        for (const target of inspectableBinaries(copyTarget)) {
          extraImages.push({ source: join(copySource, relative(copyTarget, target)), target, runpaths });
        }
      } else {
        copyFileSync(copySource, copyTarget);
      }
      console.log(`bundled ${copySource} -> ${copyTarget}`);
    }
    const target = realpathSync(targetBinary);
    if (!target.startsWith(`${frameworksDir}/`)) fail(`bundled dependency escapes Frameworks: ${targetBinary}`);
    chmodSync(target, 0o755);
    return target;
  }

  function visit(source, target, inherited) {
    const canonical = realpathSync(source);
    if (targetsBySource.has(canonical)) return targetsBySource.get(canonical);
    targetsBySource.set(canonical, target);

    const { installName, dependencies, runpaths } = machoDescription(source);
    // Expand each loader's runpaths in its original directory before moving its dependencies.
    const searchPaths = [...new Set([...runpaths.flatMap((path) => localPaths(path, source, inherited)), ...inherited])];
    const args = [];
    for (const reference of new Set(dependencies)) {
      if (systemLibrary(reference)) continue;
      const paths = localPaths(reference, source, searchPaths);
      const dependency = paths.find((path) => existsSync(path) && statSync(path).isFile());
      if (!dependency) fail(`cannot resolve ${reference} from ${source}; tried ${paths.join(", ") || "no runpaths"}`);

      let replacement = realpathSync(dependency);
      if (!systemLibrary(replacement)) {
        const staged = targetsBySource.get(replacement) ?? copyImage(dependency, searchPaths);
        visit(dependency, staged, searchPaths);
        replacement = `@loader_path/${relative(dirname(target), staged)}`;
      }
      if (reference !== replacement) args.push("-change", reference, replacement);
    }

    if (installName && target.startsWith(`${frameworksDir}/`)) {
      const replacement = `@rpath/${relative(frameworksDir, target)}`;
      if (installName !== replacement) args.push("-id", replacement);
    }
    for (const path of new Set(runpaths)) {
      if (isAbsolute(path) && !systemLibrary(path)) args.push("-delete_rpath", path);
    }
    if (args.length) edits.set(target, args);
    return target;
  }

  visit(executable, executable, []);
  for (const { source, target, runpaths } of extraImages) visit(source, target, runpaths);
  for (const [target, args] of edits) command("install_name_tool", [...args, target]);
}

if (!verifyOnly) bundleDependencies();
verifyBundle();

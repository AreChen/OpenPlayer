import assert from "node:assert/strict";
import { copyFile, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir, userInfo } from "node:os";
import { basename, join, resolve } from "node:path";
import { PackageSmokeHost } from "./package-smoke-host.mjs";
import { generateMedia, packages, run, sha256, start, waitFor, withFixture, writeEvidence } from "./package-smoke-tools.mjs";

const [baseDirectory, targetDirectory, tag, targetTag, commit, evidenceDirectory, profileHelper] = process.argv.slice(2);
if (!profileHelper) {
  throw new Error("Use run-upgrade-smoke.mjs BASE_DIR TARGET_DIR BASE_TAG TARGET_TAG COMMIT EVIDENCE_DIR PROFILE_HELPER");
}
const from = await packages(resolve(baseDirectory), tag);
const to = await packages(resolve(targetDirectory), targetTag);
assert.equal(from.commit, commit);
assert.equal(to.commit, commit);
assert.equal(from.platform, to.platform);
const oldVersion = tag.slice(1);
const newVersion = targetTag.slice(1);

async function processAlive(player) {
  assert.equal(player.child.exitCode, null, "updater unexpectedly exited the original player");
  assert.equal(player.child.signalCode, null, "updater unexpectedly stopped the original player");
}

async function awaitStatus(file, predicate) {
  return waitFor(`status matching the upgrade expectation in ${file}`, async () => {
    const text = await readFile(file, "utf8").catch((error) => {
      if (error.code === "ENOENT") return "";
      throw error;
    });
    return predicate(text);
  }, 150_000);
}

function jsonOrNull(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

const policyPath = "/etc/polkit-1/rules.d/00-openplayer-package-smoke.rules";

async function polkit(host, allow) {
  const rule = join(host.directory, "polkit-test.rules");
  const user = JSON.stringify(userInfo().username);
  await writeFile(rule, `
polkit.addRule(function(action, subject) {
  if (action.id === "org.freedesktop.policykit.exec"
      && action.lookup("program") === "/usr/bin/apt-get"
      && subject.user === ${user}) {
    return polkit.Result.${allow ? "YES" : "NO"};
  }
});
`);
  await run("sudo", ["install", "-o", "root", "-g", "root", "-m", "644", rule, policyPath]);
  await waitFor("temporary apt-get PolicyKit rule", async () => {
    const result = await start("pkexec", ["--disable-internal-agent", "/usr/bin/apt-get", "--version"], { stdio: "ignore" }).finished;
    return allow ? result.code === 0 : result.code === 126 || result.code === 127;
  }, 15_000);
}

async function attemptUpdate(host, packagePath, mode, media) {
  const file = join(host.directory, `${mode}.status`);
  let result;
  let downloadedCopy;
  const requests = await withFixture(packagePath, targetTag, mode, async (origin) => {
    let player;
    let window;
    const environment = { OPENPLAYER_TEST_UPDATE_ORIGIN: origin };
    if (process.platform === "win32") {
      const action = mode === "cancel" ? "install" : "install-silent";
      player = host.launch(["--smoke-update", action, file, media], environment);
      window = await host.window(player);
      await awaitStatus(file, (text) => {
        if (text.includes("error:") && (mode === "install" || mode === "cancel")) throw new Error(text.trim());
        return text.includes(`started:${oldVersion}`) && text.includes("playing");
      });
      if (mode === "cancel") {
        const installer = await awaitStatus(file, (text) => /^installer:(\d+)$/m.exec(text)?.[1]);
        await host.cancelWindowsInstaller(installer);
        result = await awaitStatus(file, (text) => text.includes("installer-exit:") && text.trim());
      } else if (mode === "install") {
        await awaitStatus(file, (text) => /^installer:(\d+)$/m.exec(text)?.[1]);
        await waitFor("NSIS to replace the running OpenPlayer executable", async () => {
          const current = await host.version().catch(() => null);
          const exited = player.child.exitCode !== null || player.child.signalCode !== null;
          return current === newVersion && exited;
        }, 120_000);
        result = (await readFile(file, "utf8")).trim();
        assert.ok(result.includes(`verified:${newVersion}`), "the updater did not verify the NSIS payload");
        return;
      } else {
        result = await awaitStatus(file, (text) => text.includes("error:") && text.trim());
        assert.ok(!result.includes("installer:"), "installer was invoked on an unusable download");
      }
    } else {
      const confirm = mode !== "cancel" || host.packageKind === "deb";
      player = host.launch([media], {
        ...environment,
        OPENPLAYER_TEST_UPDATE_ACTION: confirm ? "install" : "download",
        OPENPLAYER_TEST_UPDATE_RESULT: file,
      });
      window = await host.window(player);
      result = await awaitStatus(file, jsonOrNull);
      assert.equal(result.currentVersion, oldVersion);
      assert.equal(result.playing, true, "the original player was not playing at the update attempt");
      if (mode === "install") {
        assert.equal(result.status, host.packageKind === "dmg" ? "installerOpened" : "installed", result.error);
        assert.equal(result.version, newVersion);
        if (host.packageKind === "dmg") {
          downloadedCopy = join(host.directory, basename(packagePath));
          await copyFile(result.path, downloadedCopy);
          assert.equal(await sha256(downloadedCopy), await sha256(packagePath));
        }
      } else if (confirm) {
        assert.equal(result.status, "failed", JSON.stringify(result));
      } else {
        assert.equal(result.status, "ready", JSON.stringify(result));
        assert.equal(result.version, newVersion);
      }
    }
    await processAlive(player);
    const { code, signal } = await host.close(player, window);
    assert.equal(code, 0, `the original player exited with ${code ?? signal}`);
  });
  const downloads = requests.filter((path) => path.endsWith(`/${basename(packagePath)}`));
  if (mode !== "offline") {
    assert.ok(downloads.length, `the ${mode} test did not reach the actual package request`);
  } else {
    assert.equal(downloads.length, 0, `${mode} must leave the package undownloaded`);
  }
  return { result, requests, downloadedCopy };
}

async function verifyRelaunch(host, packagePath, media) {
  const file = join(host.directory, "relaunch.status");
  let result;
  await withFixture(packagePath, targetTag, "current", async (origin) => {
    const windows = process.platform === "win32";
    const player = host.launch(windows ? ["--smoke-update", "playback", file, media] : [media], {
      OPENPLAYER_TEST_UPDATE_ORIGIN: origin,
      OPENPLAYER_TEST_UPDATE_ACTION: "check",
      OPENPLAYER_TEST_UPDATE_RESULT: file,
    });
    const window = await host.window(player);
    if (windows) {
      result = await awaitStatus(file, (text) => text.includes(`started:${newVersion}`) && text.includes("playing") && text.trim());
    } else {
      result = await awaitStatus(file, jsonOrNull);
      assert.equal(result.currentVersion, newVersion);
      assert.equal(result.status, "current");
      assert.equal(result.playing, true);
    }
    const { code, signal } = await host.close(player, window);
    assert.equal(code, 0, `the upgraded player exited with ${code ?? signal}`);
  });
  return result;
}

async function wayland(host, packagePath, media) {
  const runtime = join(host.directory, "wayland");
  await mkdir(runtime, { mode: 0o700 });
  const env = { ...host.environment, XDG_RUNTIME_DIR: runtime, WAYLAND_DISPLAY: "openplayer-test" };
  delete env.DISPLAY;
  const compositor = start("weston", ["--backend=headless-backend.so", "--socket=openplayer-test", "--idle-time=0"], { env });
  host.background.push(compositor);
  await waitFor("Wayland socket", async () => (await stat(join(runtime, env.WAYLAND_DISPLAY)).catch(() => null))?.isSocket());
  const file = join(host.directory, "wayland.status");
  let result;
  const requests = await withFixture(packagePath, targetTag, "wayland", async (origin) => {
    const player = host.launch([media], {
      ...env,
      DISPLAY: "",
      GDK_BACKEND: "wayland",
      OPENPLAYER_TEST_UPDATE_ORIGIN: origin,
      OPENPLAYER_TEST_UPDATE_ACTION: "check",
      OPENPLAYER_TEST_UPDATE_RESULT: file,
    });
    result = await awaitStatus(file, jsonOrNull);
    assert.equal(result.displayServer, "wayland");
    assert.equal(result.mpvEmbedVideo, false);
    assert.equal(result.playing, false);
    await processAlive(player);
    await host.close(player);
  });
  compositor.child.kill();
  return { result, requests };
}

await mkdir(evidenceDirectory, { recursive: true });
const evidence = { tag, targetTag, commit, platform: from.platform, isolatedFeed: true, packages: [] };
try {
  for (const [index, path] of from.files.entries()) {
    const target = to.files[index];
    const name = basename(path);
    const targetName = basename(target);
    const result = {
      name,
      sha256: from.assets.find((asset) => asset.name === name).sha256,
      target: targetName,
      targetSha256: to.assets.find((asset) => asset.name === targetName).sha256,
      checks: {},
    };
    evidence.packages.push(result);
    const directory = await mkdtemp(join(tmpdir(), "openplayer-upgrade-"));
    const media = await generateMedia(directory);
    const host = new PackageSmokeHost(directory, profileHelper);
    try {
      await host.prepare();
      await host.install(path, oldVersion);
      await host.play(media.video, false);
      await host.play(media.audio, false);
      const original = await host.checksum();
      const before = await host.readProfile(media.audio, "seed");
      const priorAudio = before.history.find((entry) => basename(entry.path) === basename(media.audio));
      assert.ok(priorAudio?.position > 0, "v2.0.0 did not persist audio progress");

      if (process.platform === "linux") {
        result.checks.wayland = await wayland(host, target, media.video);
      }
      if (host.packageKind === "deb") await polkit(host, false);
      for (const mode of ["offline", "corrupt", "cancel"]) {
        result.checks[mode] = await attemptUpdate(host, target, mode, media.video);
        await host.assertVersion(oldVersion);
        assert.equal(await host.checksum(), original, `${mode} changed the original executable`);
      }
      if (host.packageKind === "deb") await polkit(host, true);

      const { downloadedCopy, ...install } = await attemptUpdate(host, target, "install", media.video);
      result.checks.install = install;
      if (host.packageKind === "dmg") {
        // macOS presents the downloaded DMG in Finder. Perform the operator's
        // drag-and-drop copy from that exact payload into the isolated folder.
        await rm(host.installDirectory, { recursive: true });
        await host.copyFromDmg(downloadedCopy);
      }
      await host.assertVersion(newVersion);
      assert.notEqual(await host.checksum(), original, "upgrade left the old executable in place");
      if (host.packageKind === "appimage") assert.equal(await host.checksum(), await sha256(target));
      result.checks.launchAfterUpgrade = await verifyRelaunch(host, target, media.video);
      const after = await host.readProfile(media.audio);
      assert.deepEqual(after.settings, before.settings, "playback settings changed during upgrade");
      assert.deepEqual(after.favorites, before.favorites, "favorites changed during upgrade");
      assert.deepEqual(after.history.find((entry) => basename(entry.path) === basename(media.audio)), priorAudio);
      result.checks.profile = { volume: after.settings.volume, historyRestored: true, favorites: after.favorites?.length ?? null };
    } catch (error) {
      result.error = String(error);
      throw error;
    } finally {
      if (host.packageKind === "deb") await run("sudo", ["rm", "-f", policyPath]);
      await host.cleanup();
    }
  }
} finally {
  await writeEvidence(join(evidenceDirectory, "isolated-upgrade.json"), evidence);
}

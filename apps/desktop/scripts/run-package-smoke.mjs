import assert from "node:assert/strict";
import { mkdir, mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { PackageSmokeHost } from "./package-smoke-host.mjs";
import { generateMedia, packages, writeEvidence } from "./package-smoke-tools.mjs";

const [candidateDirectory, tag, commit, evidenceDirectory, profileHelper] = process.argv.slice(2);
if (!profileHelper) {
  throw new Error("Use run-package-smoke.mjs PACKAGE_DIR TAG COMMIT EVIDENCE_DIR PROFILE_HELPER");
}
const candidate = await packages(resolve(candidateDirectory), tag);
assert.equal(candidate.commit, commit, "package manifest must match the verified source commit");
await mkdir(evidenceDirectory, { recursive: true });

const evidence = { tag, commit, platform: candidate.platform, distribution: candidate.distribution, packages: [] };
try {
  for (const path of candidate.files) {
    const directory = await mkdtemp(join(tmpdir(), "openplayer-installed-"));
    const media = await generateMedia(directory);
    const host = new PackageSmokeHost(directory, profileHelper);
    const name = basename(path);
    const result = { name, sha256: candidate.assets.find((asset) => asset.name === name).sha256, checks: {} };
    evidence.packages.push(result);
    try {
      await host.prepare();
      await host.install(path, tag.slice(1));
      result.checks.installed = true;
      result.checks.video = await host.play(media.video);
      const before = await host.readProfile(media.video);
      assert.ok(before.history.some((entry) => basename(entry.path) === basename(media.video) && entry.position > 0),
        "the installed player did not persist video progress");
      if (result.checks.video.controlsSent) {
        assert.ok(before.settings?.volume < 82, "the installed player did not apply the volume key");
      }
      result.checks.audio = await host.play(media.audio, false);
      const after = await host.readProfile(media.video);
      assert.ok(after.history.some((entry) => basename(entry.path) === basename(media.audio) && entry.position > 0),
        "the installed player did not persist audio progress");
      assert.deepEqual(after.settings, before.settings, "playback settings changed after relaunch");
      assert.deepEqual(
        after.history.find((entry) => basename(entry.path) === basename(media.video)),
        before.history.find((entry) => basename(entry.path) === basename(media.video)),
        "video history changed after playing another file",
      );
      result.checks.profile = { volume: after.settings?.volume ?? null, historyEntries: after.history.length };
    } catch (error) {
      result.error = String(error);
      throw error;
    } finally {
      await host.cleanup();
    }
  }
} finally {
  await writeEvidence(join(evidenceDirectory, "installed-packages.json"), evidence);
}

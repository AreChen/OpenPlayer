import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readFile, readdir, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, resolve } from "node:path";
import { Transform } from "node:stream";
import { setTimeout } from "node:timers/promises";

export async function waitFor(description, check, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await check();
    if (value) return value;
    await setTimeout(250);
  }
  throw new Error(`Timed out waiting for ${description}`);
}

export function start(command, args, { env = process.env, cwd, stdio = "inherit" } = {}) {
  const child = spawn(command, args, { cwd, env, stdio });
  const finished = new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code, signal) => resolve({ code, signal }));
  });
  // Some callers wait on the process while separately monitoring a status file.
  finished.catch(() => {});
  return { child, finished };
}

export async function run(command, args, options) {
  const { finished } = start(command, args, options);
  const { code, signal } = await finished;
  if (code !== 0) {
    throw new Error(`${command} exited with ${code ?? signal}`);
  }
}

export async function capture(command, args, options) {
  const { child, finished } = start(command, args, { ...options, stdio: ["ignore", "pipe", "pipe"] });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8").on("data", (chunk) => { stdout += chunk; });
  child.stderr.setEncoding("utf8").on("data", (chunk) => { stderr += chunk; });
  const { code, signal } = await finished;
  if (code !== 0) {
    throw new Error(`${command} exited with ${code ?? signal}\n${stderr}`.trimEnd());
  }
  return stdout.trim();
}

export async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
  }
  return hash.digest("hex");
}

export async function packages(directory, tag) {
  const names = await readdir(directory);
  const build = JSON.parse(await readFile(resolve(directory, "build.json"), "utf8"));
  const { commit } = build;
  if (build.tag !== tag || !/^[0-9a-f]{40}$/.test(commit)) {
    throw new Error(`Build metadata in ${directory} does not match ${tag}`);
  }
  const suffixes = {
    win32: ["_x64-setup.exe"],
    darwin: [process.arch === "arm64" ? "_arm64.dmg" : "_x64.dmg"],
    linux: ["_amd64.deb", "_amd64.AppImage"],
  }[process.platform];
  if (!suffixes) throw new Error(`Unsupported package smoke platform: ${process.platform}`);
  const files = suffixes.map((suffix) => `OpenPlayer_${tag.slice(1)}${suffix}`);
  for (const name of files) {
    const path = resolve(directory, name);
    if (!names.includes(name)) throw new Error(`Missing package: ${path}`);
    const checksum = (await readFile(`${path}.sha256`, "utf8")).trim();
    const hash = await sha256(path);
    if (checksum !== `${hash}  ${name}` || !build.assets.some((asset) => asset.name === name && asset.sha256 === hash)) {
      throw new Error(`Package checksum mismatch: ${path}`);
    }
  }
  return { ...build, files: files.map((name) => resolve(directory, name)) };
}

export async function generateMedia(directory) {
  const video = resolve(directory, "sample video.mp4");
  const audio = resolve(directory, "sample audio.wav");
  const args = ["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi"];
  await run("ffmpeg", [
    ...args, "-i", "testsrc2=duration=90:size=320x180:rate=15",
    "-f", "lavfi", "-i", "sine=frequency=440:duration=90",
    "-c:v", "libx264", "-c:a", "aac", video,
  ]);
  await run("ffmpeg", [...args, "-i", "sine=frequency=330:duration=30", audio]);
  return { video, audio };
}

export async function withFixture(path, tag, mode, body) {
  const name = basename(path);
  const checksum = `${path}.sha256`;
  const hashPath = `/AreChen/OpenPlayer/releases/download/${tag}/${name}.sha256`;
  const assetPath = `/AreChen/OpenPlayer/releases/download/${tag}/${name}`;
  const sizes = await Promise.all([stat(path), stat(checksum)].map(async (file) => (await file).size));
  const requests = [];
  const server = createServer((request, response) => {
    requests.push(new URL(request.url, "http://127.0.0.1").pathname);
    if (mode === "offline") {
      request.socket.destroy();
      return;
    }
    const url = new URL(request.url, "http://127.0.0.1");
    if (url.pathname === "/repos/AreChen/OpenPlayer/releases") {
      const releases = url.searchParams.get("page") === "1" ? [{
        tag_name: tag,
        draft: false,
        prerelease: false,
        assets: [name, `${name}.sha256`].map((asset, index) => ({
          name: asset, state: "uploaded", size: sizes[index],
        })),
      }] : [];
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify(releases));
      return;
    }
    const source = url.pathname === assetPath ? path : url.pathname === hashPath ? checksum : null;
    if (!source) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, {
      "Content-Type": "application/octet-stream",
      "Content-Length": url.pathname === assetPath ? sizes[0] : sizes[1],
    });
    let stream = createReadStream(source);
    stream.once("error", (error) => response.destroy(error));
    if (source === path && mode === "corrupt") {
      let corruptFirstChunk = true;
      stream = stream.pipe(new Transform({
        transform(data, _encoding, done) {
          if (corruptFirstChunk) {
            data[0] ^= 0xff;
            corruptFirstChunk = false;
          }
          done(null, data);
        },
      }));
    }
    stream.pipe(response);
    response.once("close", () => stream.destroy());
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  try {
    await body(`http://127.0.0.1:${server.address().port}`);
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
  return requests;
}

export async function writeEvidence(path, evidence) {
  await writeFile(path, `${JSON.stringify(evidence, null, 2)}\n`);
}

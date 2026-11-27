import { chmod, copyFile, mkdir, readdir, rm, stat, writeFile } from "node:fs/promises";
import { basename, dirname, extname, join, resolve } from "node:path";
import { setTimeout } from "node:timers/promises";
import { capture, run, sha256, start, waitFor } from "./package-smoke-tools.mjs";

const powershell = String.raw`$ErrorActionPreference = 'Stop'; `;

async function windows(script, variables) {
  return capture("pwsh", ["-NoProfile", "-NonInteractive", "-Command", powershell + script], {
    env: { ...process.env, ...variables },
  });
}

async function attempt(command, args, options) {
  try {
    return await capture(command, args, options);
  } catch {
    return "";
  }
}

const macToolSource = `
import AppKit
import CoreGraphics

let action = CommandLine.arguments[1]
let pid = pid_t(CommandLine.arguments[2])!
guard let app = NSRunningApplication(processIdentifier: pid) else { exit(1) }

switch action {
case "window":
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
        as? [[String: Any]] ?? []
    guard list.contains(where: {
        ($0[kCGWindowOwnerPID as String] as? Int) == Int(pid)
            && ($0[kCGWindowLayer as String] as? Int) == 0
    }) else { exit(1) }
case "close":
    guard app.terminate() else { exit(1) }
case "keys":
    guard CGPreflightPostEventAccess() else { exit(1) }
    let deadline = Date(timeIntervalSinceNow: 3)
    if NSWorkspace.shared.frontmostApplication?.processIdentifier != pid {
        guard app.activate(options: [.activateAllWindows]) else { exit(1) }
    }
    while NSWorkspace.shared.frontmostApplication?.processIdentifier != pid && Date() < deadline {
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.1))
    }
    for key in [49, 124, 125, 49] {
        guard NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { exit(1) }
        CGEvent(keyboardEventSource: nil, virtualKey: CGKeyCode(key), keyDown: true)?.post(tap: .cghidEventTap)
        CGEvent(keyboardEventSource: nil, virtualKey: CGKeyCode(key), keyDown: false)?.post(tap: .cghidEventTap)
        Thread.sleep(forTimeInterval: 0.2)
    }
default:
    exit(2)
}
print("ok")
`;

export class PackageSmokeHost {
  constructor(directory, profileHelper) {
    this.directory = directory;
    this.profileHelper = resolve(profileHelper);
    this.environment = { ...process.env };
    this.background = [];
    this.profile = join(directory, "profile");
  }

  async prepare() {
    await mkdir(this.profile, { recursive: true });
    switch (process.platform) {
      case "win32":
        this.environment.APPDATA = this.profile;
        this.environment.OPENPLAYER_WINDOWS_SMOKE_PROFILE = "packages";
        this.storage = join(this.profile, "dev.openplayer.gpui-smoke-packages", "storage");
        await windows(String.raw`
          if (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPlayer') {
            throw 'OpenPlayer is already installed in this Windows test account.'
          }
        `);
        break;
      case "linux": {
        this.environment.XDG_CONFIG_HOME = join(this.profile, "config");
        this.environment.XDG_DATA_HOME = join(this.profile, "data");
        this.environment.XDG_CACHE_HOME = join(this.profile, "cache");
        this.storage = join(this.environment.XDG_DATA_HOME, "dev.openplayer.app", "storage");
        const listed = await Promise.all(
          ["openplayer", "open-player"].map((name) => attempt("dpkg-query", ["-W", "-f=${db:Status-Status}", name])),
        );
        if (listed.includes("installed")) throw new Error("OpenPlayer is already installed on this Linux test host.");
        const desktop = start("openbox", [], { env: this.environment });
        this.background.push(desktop);
        await waitFor("X11 window manager", () => attempt("wmctrl", ["-m"]));
        break;
      }
      case "darwin": {
        this.environment.HOME = this.profile;
        this.storage = join(this.profile, "Library", "Application Support", "dev.openplayer.desktop", "storage");
        const source = join(this.directory, "package-smoke.swift");
        this.macTool = join(this.directory, "package-smoke");
        await writeFile(source, macToolSource);
        await run("swiftc", [source, "-o", this.macTool]);
        break;
      }
      default:
        throw new Error(`Unsupported test platform: ${process.platform}`);
    }
  }

  async install(path, version) {
    switch (extname(path)) {
      case ".exe": {
        this.installDirectory = join(this.directory, "installed");
        await windows(String.raw`
          $setup = Start-Process -FilePath $env:OPENPLAYER_SETUP -ArgumentList "/S /D=$env:OPENPLAYER_INSTALL" -PassThru -Wait
          if ($setup.ExitCode -ne 0) { throw "NSIS setup exited with $($setup.ExitCode)" }
        `, { OPENPLAYER_SETUP: path, OPENPLAYER_INSTALL: this.installDirectory });
        this.binary = join(this.installDirectory, "openplayer-desktop.exe");
        this.packageKind = "nsis";
        await stat(join(this.installDirectory, "libmpv-2.dll"));
        await stat(join(this.installDirectory, "licenses", "libmpv-LICENSE.LGPL"));
        const contents = await readdir(this.installDirectory);
        if (contents.some((name) => /webview|tauri|wry/i.test(name))) {
          throw new Error(`Unexpected web runtime in NSIS install: ${contents.join(", ")}`);
        }
        break;
      }
      case ".deb":
        this.packageKind = "deb";
        this.deb = await capture("dpkg-deb", ["--field", path, "Package"]);
        if (!["openplayer", "open-player"].includes(this.deb)) {
          throw new Error(`Unexpected Debian package name: ${this.deb}`);
        }
        await run("sudo", ["apt-get", "install", "-y", path]);
        this.binary = (await capture("dpkg", ["-L", this.deb]))
          .split("\n").find((file) => /^\/usr\/bin\/openplayer(-desktop)?$/.test(file));
        if (!this.binary) throw new Error("The Debian package did not install the player executable.");
        break;
      case ".AppImage":
        this.packageKind = "appimage";
        this.binary = join(this.directory, "OpenPlayer.AppImage");
        await copyFile(path, this.binary);
        await chmod(this.binary, 0o755);
        break;
      case ".dmg":
        this.packageKind = "dmg";
        this.installDirectory = join(this.directory, "Applications", "OpenPlayer.app");
        this.binary = join(this.installDirectory, "Contents", "MacOS", "openplayer-desktop");
        await this.copyFromDmg(path);
        break;
      default:
        throw new Error(`Unsupported package: ${path}`);
    }
    await this.assertVersion(version);
    return this.binary;
  }

  async copyFromDmg(path) {
    const mount = join(this.directory, "dmg");
    await mkdir(mount, { recursive: true });
    await mkdir(dirname(this.installDirectory), { recursive: true });
    await run("hdiutil", ["attach", "-nobrowse", "-readonly", "-mountpoint", mount, path]);
    try {
      await run("ditto", [join(mount, "OpenPlayer.app"), this.installDirectory]);
      await run("codesign", ["--verify", "--deep", "--strict", this.installDirectory]);
    } finally {
      await run("hdiutil", ["detach", mount]);
    }
  }

  async assertVersion(expected) {
    await stat(this.binary);
    const actual = await this.version();
    if (actual && actual !== expected) throw new Error(`Installed ${this.packageKind} version ${actual}, expected ${expected}`);
  }

  async version() {
    if (this.packageKind === "nsis") {
      return windows(String.raw`
        $install = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPlayer'
        if ($install.InstallLocation -ne $env:OPENPLAYER_INSTALL) { throw 'Unexpected OpenPlayer install location' }
        $install.DisplayVersion
      `, { OPENPLAYER_INSTALL: this.installDirectory });
    }
    if (this.packageKind === "deb") return capture("dpkg-query", ["-W", "-f=${Version}", this.deb]);
    if (this.packageKind === "dmg") {
      return capture("/usr/libexec/PlistBuddy", [
        "-c", "Print CFBundleShortVersionString", join(this.installDirectory, "Contents", "Info.plist"),
      ]);
    }
    return null;
  }

  launch(args, environment = {}) {
    const env = { ...this.environment, ...environment };
    if (this.packageKind === "appimage") {
      // Native FUSE mounts are unavailable on some hosted Linux runners.
      if (process.env.OPENPLAYER_SMOKE_APPIMAGE_EXTRACT === "1") env.APPIMAGE_EXTRACT_AND_RUN = "1";
    }
    const player = start(this.binary, args, { env });
    this.background.push(player);
    return player;
  }

  async window(player) {
    const { child } = player;
    const check = async () => {
      if (child.exitCode !== null || child.signalCode !== null) {
        throw new Error(`Player closed before it created a window (exit ${child.exitCode}, signal ${child.signalCode})`);
      }
      if (process.platform === "win32") {
        return windows(String.raw`
          $p = Get-Process -Id $env:OPENPLAYER_PID -ErrorAction SilentlyContinue
          if ($p -and $p.Responding -and $p.MainWindowTitle -eq 'OpenPlayer') { $p.MainWindowHandle }
        `, { OPENPLAYER_PID: String(child.pid) });
      }
      if (process.platform === "darwin") return attempt(this.macTool, ["window", String(child.pid)]);
      return (await attempt("xdotool", ["search", "--onlyvisible", "--name", "^OpenPlayer$"])).split("\n").at(-1);
    };
    return waitFor("visible OpenPlayer window", check);
  }

  async exerciseControls(player, window) {
    if (process.platform === "win32") {
      await windows(String.raw`
        $controller = New-Object -ComObject WScript.Shell
        if (-not $controller.AppActivate([int]$env:OPENPLAYER_PID)) { throw 'Could not focus OpenPlayer' }
        foreach ($key in @(' ', '{RIGHT}', '{DOWN}', ' ')) {
          $controller.SendKeys($key)
          Start-Sleep -Milliseconds 200
        }
      `, { OPENPLAYER_PID: String(player.child.pid) });
      return true;
    }
    if (process.platform === "darwin") return Boolean(await attempt(this.macTool, ["keys", String(player.child.pid)]));
    await run("xdotool", ["windowactivate", "--sync", window, "key", "--clearmodifiers", "--delay", "200", "space", "Right", "Down", "space"]);
    return true;
  }

  async close(player, window) {
    if (player.child.exitCode !== null) throw new Error("OpenPlayer exited before the smoke test requested closure");
    if (process.platform === "win32") {
      await windows(String.raw`
        if (-not (Get-Process -Id $env:OPENPLAYER_PID).CloseMainWindow()) {
          throw 'OpenPlayer did not accept its close request'
        }
      `, { OPENPLAYER_PID: String(player.child.pid) });
    } else if (process.platform === "darwin") {
      await run(this.macTool, ["close", String(player.child.pid)]);
    } else if (window) {
      await run("wmctrl", ["-ic", `0x${Number(window).toString(16)}`]);
    } else {
      player.child.kill();
    }
    await waitFor("OpenPlayer process to exit", () => player.child.exitCode !== null || player.child.signalCode !== null);
    return player.finished;
  }

  async cancelWindowsInstaller(pid) {
    await waitFor("NSIS confirmation window", () => windows(String.raw`
      $setup = Get-Process -Id $env:OPENPLAYER_PID -ErrorAction SilentlyContinue
      if ($setup -and $setup.MainWindowHandle -ne [IntPtr]::Zero -and $setup.CloseMainWindow()) { 'sent' }
    `, { OPENPLAYER_PID: pid }));
  }

  async checksum() {
    return sha256(this.binary);
  }

  async readProfile(media, action = "read") {
    return JSON.parse(await capture(this.profileHelper, [action, this.storage, media]));
  }

  async cleanup() {
    for (const { child } of this.background.reverse()) {
      if (child.exitCode === null && child.signalCode === null) child.kill();
    }
    switch (this.packageKind) {
      case "nsis": {
        const uninstaller = join(this.installDirectory, "uninstall.exe");
        await windows(String.raw`
          $result = Start-Process -FilePath $env:OPENPLAYER_UNINSTALL -ArgumentList '/S' -PassThru -Wait
          if ($result.ExitCode -ne 0) { throw "NSIS uninstall exited with $($result.ExitCode)" }
        `, { OPENPLAYER_UNINSTALL: uninstaller });
        await waitFor("NSIS uninstaller to remove the executable", async () => {
          if (await stat(this.binary).catch(() => null)) return false;
          return true;
        });
        break;
      }
      case "deb":
        if (this.deb) await run("sudo", ["apt-get", "remove", "-y", this.deb]);
        break;
      case "appimage":
        await rm(this.binary, { force: true });
        break;
      case "dmg":
        await rm(this.installDirectory, { recursive: true, force: true });
        break;
    }
  }

  async play(media, controlsRequested = true) {
    const player = this.launch([media]);
    const window = await this.window(player);
    await waitFor(`persisted playback progress for ${basename(media)}`, async () => {
      if (player.child.exitCode !== null || player.child.signalCode !== null) {
        throw new Error("OpenPlayer exited before recording playback progress");
      }
      // The running player briefly owns the database while writing progress.
      const profile = await this.readProfile(media).catch(() => null);
      return profile?.history.some((entry) => basename(entry.path) === basename(media) && entry.position > 0);
    }, 35_000);
    const controls = controlsRequested && await this.exerciseControls(player, window);
    await setTimeout(1_500);
    const { code, signal } = await this.close(player, window);
    if (code !== 0) throw new Error(`OpenPlayer exited with ${code ?? signal}`);
    return { window: true, controlsSent: controls, cleanExit: true };
  }
}

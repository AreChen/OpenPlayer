# Host-owned runtime copies

The Windows developer window harness now loads its embedded video runtime from
a verified, process-owned copy rather than an installed plugin directory.
**This is compiled only for tests and `window-smoke`, not exposed through Tauri
or the public plugin SDK.** Normal player startup and published 1.6.3 are unchanged.

## Ownership and limits

`src-tauri/src/native_runtime/` separates inventory verification, cache ownership
and the Windows VSScript loader. The cache layer is not tied to an effect/vendor.
The current loader specifically supports the proven private CPython 3.12 /
VapourSynth layout; it is not an arbitrary public DLL-loading API.

- Inventory schema 2 contains `schema`, optional bounded `metadata`, and `files`
  mapping relative paths to `size` and `sha256`. Effect-specific metadata is not
  used as a host permission or feature gate.
- The inventory is limited to 256 KiB, total payload plus inventory to 128 MiB,
  and payload files to 1023. Traversal depth/directory entries are bounded too.
  Unknown envelope fields, case-colliding names, device/stream/traversal paths,
  reparse points, undeclared files and hash mismatches are rejected.
- Every copy uses a fresh host-cache directory and streams files through a hash
  check with declared byte limits. It never overwrites an existing copy. Failed
  copies/loads are cleaned without modifying the source package.
- A Windows `.lease` file denies concurrent read/write opens while its owning
  process is alive. The next preparation scans at most 64 cache-root entries
  and removes only recognizable, unlocked runtime directories. Unrelated names
  and live leases are preserved; cleanup errors are reported.
- One inventory fingerprint is resident per process. Identical inventories reuse
  that copy; a different fingerprint reports that a restart is required. The
  host does not restart automatically. This intentionally compares the complete
  bundle inventory, not individual library versions.
- Existing `VSSCRIPT_PATH` or preloaded unmanaged Python/VapourSynth libraries
  cause an explicit conflict. The loader changes neither process environment nor
  DLL search paths. Python's private `_pth` ignores outside Python paths.
- After a successful load, the library and lease remain resident until OS process
  teardown. The host does not call `FreeLibrary` or attempt Python reinitialization.
  Disk copies remain until a later preparation reclaims them; they are not tied
  to plugin uninstall or a view's close action.

These bounds and fingerprints are correctness checks, not code signing or an OS
sandbox. Same-user native code is trusted and can modify files. Production callers
must still resolve installed declarations, check consent/lifecycle generations,
choose host-owned cache paths and serialize video attachment ownership.

## Verify the actual window path

Build the host from this repository:

```powershell
rtk proxy cargo build -p openplayer-desktop --features window-smoke --bin window-smoke
```

From the sibling `openplayer-dlssnr` repository, rebuild the portable directory
with its current builder (older schema-1 inventories must be rebuilt), then run:

```powershell
rtk proxy .venv/Scripts/python.exe scripts/verify-host-runtime.py .local/portable-module --window-smoke ../RustPlayer/target/debug/window-smoke.exe --video .local/motion-1080p-long.mp4 --report .local/host-runtime-check.json
```

The runner provides explicit test-only bundle/cache paths, captures screenshots,
and uses the existing kill-on-close supervisor. It launches two actual OpenPlayer
window harnesses sequentially. Each must reuse one cached runtime through filter
rebuilds, reject a conflicting inventory, display nonblack 1080p frames, retain
the overlay, pass pause/seek/maximized-fullscreen checks, and close both windows
with one Alt+F4. The second process must reclaim the first process's copy. Source
and cached payload hashes and the user's VapourSynth configuration are checked.
The final process's now-stale copy is intentionally retained for the next launch.

Evidence: `openplayer-dlssnr/.local/host-owned-runtime.json` passed both runs on
2026-09-08. Source and user configuration were unchanged. Unit tests separately
cover removing the source while a copy lease is alive, stale/live lease handling,
failed DLL-load cleanup, invalid paths, corrupt/extra files and unrelated cache
entries. The host suite passed 201 tests with three explicitly gated tests ignored.

## Session attachment follow-up

The next development batch connects a single cleanup lease to each native session.
Stop/prune remove the owned filter before forgetting the session; failed removal
retains tracking for retry. Idle parent exit schedules cleanup on a blocking task.
Only the Windows harness can install a real attachment; normal SDK startup cannot.
It uses an isolated package, host-cached adapter and unique owned filter labels.
No permission or Tauri command was added.

The real-window stop/reconnect scenario passed on 2026-09-09. A subsequent disable
scenario produced recovery screenshots but timed out before verified completion;
the cause remains unresolved. Further foreground runs paused while the user was
gaming. GPU-1 headless processing and real-store disable/replacement/uninstall
passed separately, not as a combined window acceptance test. See the prototype's
`docs/host-attachment.md` and `docs/gpu-selection.md` for evidence and reproduction.

## Remaining attachment work

This fixture passes original frames through; it is not a combined DLSSNR plugin
UI/SDK attachment test. Real React controls, native launch consent, an active
plugin uninstall during enhancement, PTS/A-V synchronization and a public attach /
detach operation remain unverified or unimplemented. The loader alone must not
make `native.video.validatePlan()` executable. No new permission or JS API was
introduced. The next step is completing the combined window lifecycle matrix and
production authorization/ownership integration before exposing executable SDK
attachment.

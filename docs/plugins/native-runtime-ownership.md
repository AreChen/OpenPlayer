# Host-owned runtime copies

The Windows developer window harness now loads its embedded video runtime from
a verified, process-owned copy rather than an installed plugin directory.
**The current development host also uses this for authorized
[`vapoursynth-rgb-v1` attachment](native-video.md).** The experimental
`present-rgba-v1` path does not load the XeFG vendor runtime through this cache;
that runtime stays in the native presenter process. It can consume an upstream
NR filter, but the combination is unverified. The presenter's real-host tests
cover output restoration after detach/crash and process exit, including close
during initialization; see [current evidence](native-video.md#host-verification-2026-09-12).
These results do not validate NR composition or complete an installable XeFG
product. Published 1.6.3 is unchanged.
The historical evidence below predates that public development interface.

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

The session attachment layer connects a single cleanup lease to each native session.
Stop/prune remove the owned filter before forgetting the session; failed removal
retains tracking for retry. Idle parent exit schedules cleanup on a blocking task.
The initial milestone only allowed the Windows harness to install an attachment.
It uses an isolated package, host-cached adapter and unique owned filter labels.
No permission or Tauri command was added.

On 2026-09-09 all six combined real-window scenarios passed with NR explicitly on
GPU 1: stop/reconnect, disable, upgrade, uninstall, worker crash and application
exit while enhancement remained active. The user authorized foreground tests.
Evidence: the prototype's `.local/attachment-matrix-final.json`; source and user
configuration remained unchanged. The earlier disable timeout did not recur in
an isolated repeat or subsequent matrices; its original cause remains unknown.
See the prototype's `docs/host-attachment.md` and `docs/gpu-selection.md` for
reproduction and GPU identity checks.

Application shutdown now reuses the stop-all lifecycle, including job waits and
attachment cleanup, instead of only terminating jobs and clearing the registry.
Cleanup failures are logged and remain tracked. A gated protocol integration test
verifies the shutdown callback, and the window test checks zero remaining job
processes/sessions/scripts after active-enhancement exit. The harness uses
`run_return` so event-loop completion is observable and its temporary WebView
directory can be cleaned; it rejects held modifiers before injecting Alt+F4.
The host suite passed 203 tests (three gated tests ignored), and the protocol
lifecycle test passed when explicitly enabled.

## Remaining attachment work

The combined fixture processes frames through the NR worker, but it is not a
production plugin UI/SDK attachment test. Real React controls, native launch
consent, media/format changes, PTS/A-V synchronization and a public attach/detach
operation were unverified or unimplemented at that milestone. The loader alone must not
make `native.video.validatePlan()` executable. No new permission or JS API was
introduced in that milestone. The subsequent [native video integration](native-video.md)
adds permissioned attachment and the independent plugin's enable/disable/GPU view.
Its GUI authorization acceptance and sustained A/V performance still need testing.

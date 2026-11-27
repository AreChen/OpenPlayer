# Native runtimes

| Platform | libmpv source |
| --- | --- |
| Windows x64 | Official LGPLv3 build, restored from [the pinned artifact metadata](mpv-windows-x64.json). |
| Linux | The `.deb` depends on the distribution's `libmpv2` package. The AppImage bundles `libmpv.so.2`. |
| macOS | Both architecture-specific DMGs bundle Homebrew libmpv and its dependency dylibs. |

From the repository root on Windows, run `./apps/desktop/scripts/restore-windows-mpv.ps1` to restore libmpv, the import library, headers and the license. The script verifies the archive and DLL SHA256 hashes. CI uses the same restore script. The NSIS installer includes the DLL and license. [Windows build instructions](../windows-gpui.md)

Native binaries belong in the ignored `vendor/native/` directory. Keep provenance, hashes and license information in the tracked artifact metadata. The [dependency mirror notes](mpv-windows-x64-mirror.md) record the earlier OpenPlayer 1.6.x runtime.

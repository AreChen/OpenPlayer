# Windows mpv dependency archive

Build dependency only, not an OpenPlayer installer or application update.

- Upstream: https://github.com/zhongfly/mpv-winbuild
- Upstream release: `2026-06-22-2bd9c3229f`
- Original asset: `mpv-dev-lgpl-x86_64-20260622-git-2bd9c3229f.7z`
- Original archive SHA256: `44d3d86baa3f90277c7c91fb2a651b0894f79148cce3196dc17844f9fb4b6ac3`
- Repacked archive: `openplayer-mpv-lgpl-x64-20260622.7z`
- Repacked archive SHA256: `6968d5e0437ea1878853de4ed1b05b9f13505e73a01e64eda564a44ad4902a7d`
- Runtime DLL SHA256: `391a966c84a3dc4cb4644e4e42d6fc1d3f8adbad6915deaf106fda2020628adf`
- License recorded for the upstream build: LGPL-2.1-or-later.

The original release URL returned 404. This archive preserves the locally
retained runtime, import library and four public mpv headers from that build.
The DLL hash matches the user-accepted OpenPlayer NR 0.4.0 test delivery.
No executable was rebuilt or modified. This is not a byte-identical mirror of
the original archive and does not contain a complete upstream development kit.

Upstream source and build recipes: [mpv](https://github.com/mpv-player/mpv),
[mpv-winbuild](https://github.com/zhongfly/mpv-winbuild). These components retain
their upstream licenses; OpenPlayer's application license does not replace them.

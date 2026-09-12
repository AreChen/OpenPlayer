# Native Dependencies

Official OpenPlayer releases will bundle native media dependencies per platform.

The first native dependency target is `libmpv`. Each bundled native dependency must document:

- Dependency name.
- Upstream source.
- Version.
- License.
- Platform artifact name.
- Checksum.

Large native binaries are not committed to git. Packaging scripts and metadata are tracked instead.

Tracked dependency metadata:

- `mpv-windows-x64.json` - Windows x64 mpv build used by release automation.

The Windows release manifest uses the `mpv-dev-lgpl` build from
`zhongfly/mpv-winbuild`, not the default GPL artifact. Its June 22, 2026 upstream
release was no longer available when preparing OpenPlayer 1.6.4.

The replacement download is an OpenPlayer-hosted archive containing the same
accepted `libmpv-2.dll`, import library and headers. It does not upgrade or rebuild
mpv. The archive hash changed because these extracted files were repackaged;
the manifest retains the original URL/hash and the runtime DLL hash for provenance.
The dependency is a separate prerelease, not a player update. See
[dependency archive details](mpv-windows-x64-mirror.md).

Linux packages depend on the distribution's `libmpv2` package instead of
bundling a private copy. macOS release automation currently bundles Homebrew
`mpv` dylibs into the DMG; Homebrew's mpv formula carries GPL/LGPL licensing, so
macOS runtime licensing should be reviewed before treating signed macOS releases
as permissive-only binary distributions.

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

The Windows release manifest intentionally points at the `mpv-dev-lgpl` artifact
from `zhongfly/mpv-winbuild`. Upstream documents this artifact as an
LGPLv2.1-compatible libmpv build, which is a better fit for OpenPlayer's MIT
application code than the default GPL mpv build.

Linux packages depend on the distribution's `libmpv2` package instead of
bundling a private copy. macOS release automation currently bundles Homebrew
`mpv` dylibs into the DMG; Homebrew's mpv formula carries GPL/LGPL licensing, so
macOS runtime licensing should be reviewed before treating signed macOS releases
as permissive-only binary distributions.

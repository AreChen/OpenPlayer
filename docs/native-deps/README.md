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

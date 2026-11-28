# libmpv playback checks

The native player's thumbnail renderer test decodes an actual odd-sized YUV fixture with libmpv. Run it from the repository root after installing the [native dependencies](../native-deps/README.md).

~~~sh
cargo test --locked -p openplayer-desktop renders_thumbnail_from_odd_sized_chroma_subsampled_video
~~~

For playback, use the [installed-package workflow](../../.github/workflows/package-validation.yml). It opens generated video and audio in the installed player and checks progress, controls and saved history. The Linux updater fixture separately verifies frame presentation over Wayland. [Window and playback checklist](regression-checks.md)

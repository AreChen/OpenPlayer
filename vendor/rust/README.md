# Vendored Rust dependencies

`openplayer-native-sdk-0.1.0` is the unmodified `cargo package --locked --no-verify`
output from the official `openplayer-plugins` repository, commit
`6b03746401ebbabd4ab6ea1e5cdc27ddcc17f67c`, package `packages/native-sdk`.
Its `.cargo_vcs_info.json` records the source revision. This snapshot keeps host
builds reproducible without a sibling checkout or an unpublished Git dependency.

Make SDK changes upstream, run its tests, then regenerate this package snapshot.
Do not edit the vendored implementation independently of the plugin SDK.

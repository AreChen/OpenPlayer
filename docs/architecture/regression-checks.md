# Native player checks

Run the Rust tests and Clippy on each target. Use a disposable OS account or profile when validating installation and file associations. The [package workflow](../../.github/workflows/package-validation.yml) exercises installed copies and records evidence for the candidate binaries.

- **Open and browse.** Double-click the empty window. Navigate using the editable address, play from a thumbnail, save a folder and a media favorite, then relaunch. Check the last folder, sort, category, media filter and favorites. Open several media files together and check the playlist.
- **Play.** Try video, pure audio, subtitles and a stream. Verify moving frames, audio, seek, pause, frame stepping, track selection, volume and media properties. Check that narrow controls fit and the pointer and controls hide after inactivity.
- **Window.** Play portrait and landscape footage, drag-resize, then press Ctrl+H to resume fitting. Check dragging from the stage, fullscreen, titlebar actions, focus and keyboard input. On Linux verify both X11 and Wayland. On macOS also open a file from Finder with the application already running.
- **Update.** Exercise the installed package on an isolated release feed. Verify the original package survives a failed download, bad checksum or cancellation. Install the next fixture and confirm playback, favorites and history after relaunch. Windows uses NSIS, macOS opens the downloaded DMG, and Linux tests both apt and AppImage replacement. [Harness](../../apps/desktop/scripts/run-upgrade-smoke.mjs)

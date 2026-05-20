# OpenPlayer Icon And Logo Design

## Goal

Move `openplayer_logo_10001000.png` out of the repository root and use it as the source image for OpenPlayer branding from the app asset tree. The image should become the application icon, Windows taskbar icon, installer icon, and the in-app logo shown by the player shell.

## Current State

- The temporary root source asset is `openplayer_logo_10001000.png` at `1000x1000`.
- `apps/desktop/src-tauri/icons/icon.png` is currently a `1x1` placeholder.
- Windows builds use `apps/desktop/src-tauri/icons/icon.ico` through `build.rs`.
- The React player empty state previously showed text only: `Open media` and `MPV native playback`.

## Design

- Move the root PNG to `apps/desktop/src/assets/openplayer-logo.png` and treat that app asset as the canonical source image.
- Do not leave a copy of `openplayer_logo_10001000.png` in the repository root after implementation.
- Generate the platform icon outputs under `apps/desktop/src-tauri/icons/` from the app asset source.
- Keep the existing Tauri build path stable by preserving `icons/icon.ico` and `icons/icon.png` as generated outputs.
- Add a frontend-accessible copy of the logo under `apps/desktop/src/assets/` for React UI use.
- Display the logo centered in the empty player state above the existing `Open media` call to action.
- Remove the old `MPV native playback` tagline from the empty player state.
- Do not add a new runtime dependency for image processing; use local tooling available in the environment for conversion.

## Generated Assets

The implementation should generate at least:

- `apps/desktop/src-tauri/icons/icon.png`
- `apps/desktop/src-tauri/icons/icon.ico`
- `apps/desktop/src/assets/openplayer-logo.png`

If the Tauri CLI expects additional common icon sizes, they can be generated as derived files from the same source image.

## Validation

- Verify generated image dimensions are reasonable for app icon use.
- Run the existing shell guard: `npm run verify:shell` from `apps/desktop`.
- Run the frontend build: `npm run build` from `apps/desktop`.
- If feasible, run a Tauri build to confirm Windows resource embedding still works.

## Out Of Scope

- Renaming the product.
- Redesigning the player controls.
- Changing mpv playback architecture.

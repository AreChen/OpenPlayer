# Documentation Agent Guide

This file applies to documentation under `docs/`. It exists so AI agents and
developers keep OpenPlayer plugin SDK documentation aligned with the real host
implementation.

## Primary References

- Start with `docs/plugins/sdk-1.5-developer-guide.md` for current SDK usage.
- Keep `docs/plugins/README.md` as the host/plugin architecture overview.
- Treat `docs/plugins/sdk-v1-design.md` as design history, not the current API
  contract.
- Release notes live in `docs/releases/`.

## Plugin SDK Documentation Rules

- Document only capabilities that exist in the host bridge and backend
  allowlists.
- Do not document direct Tauri access, direct filesystem access, raw sockets,
  arbitrary mpv commands, arbitrary filter graphs, or frontend-owned persistent
  playback state as supported plugin behavior.
- State clearly that plugin runtime storage, manifests, settings, and plugin
  enablement are backend-owned through `appearance_store`.
- State clearly that playback history, network stream history, resume
  positions, and playback settings are backend-owned through `playback_store`.
- Runtime events must be described as manifest-declared and explicitly
  subscribed; do not imply plugins receive every event by default.
- mpv controls must be described as permissioned, allowlisted, and
  snapshot-returning when they affect playback.
- Prefer high-level SDK APIs in examples. Use `openplayer.mpv.*` only when the
  example specifically needs mpv behavior.
- Every example that uses optional features should show
  `openplayer.capabilities.has(...)` or equivalent capability detection.

## Files To Keep In Sync

When changing SDK docs, inspect the matching implementation:

- Host constants:
  `apps/desktop/src/app/pluginRuntime/constants.ts`
- Worker bridge:
  `apps/desktop/src/app/pluginRuntime/workerSource/`
- Custom view bridge:
  `apps/desktop/src/app/pluginRuntime/viewDocument.ts`
- Runtime command handlers:
  `apps/desktop/src/hooks/pluginRuntimeCommands/`
- Manifest validation:
  `apps/desktop/src-tauri/src/appearance_store/manifest/`
- Runtime source DTOs:
  `apps/desktop/src-tauri/src/appearance_store/types/runtime.rs`
- mpv plugin backend:
  `apps/desktop/src-tauri/src/mpv_embed/plugin_core.rs`
  and `apps/desktop/src-tauri/src/mpv_embed/commands/playback/plugins.rs`
- Official SDK types and examples in the sibling `openplayer-plugins`
  repository, especially `packages/sdk/index.d.ts`,
  `packages/sdk/README.md`, `plugins/`, `templates/`, and `tests/`.

If these files disagree with the docs, fix the docs or the implementation in
the same change. Do not leave aspirational SDK behavior documented as current.

## Writing Style

- Keep docs concise and practical. Prefer runnable snippets and manifest
  fragments over broad product language.
- Use ASCII unless quoting existing localized text.
- Use stable names such as `SDK 1.5`, `webviewJs`, `openplayer-worker`, and
  `.opplugin` consistently.
- Link to existing docs instead of duplicating long sections.

## Verification

For documentation-only changes, run:

```powershell
git diff --check
```

If documentation changes the SDK contract, examples, manifests, or official
plugin behavior, also run the relevant host and plugin verification commands
from the root `AGENTS.md`.

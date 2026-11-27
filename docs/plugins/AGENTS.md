# Plugin documentation guide

SDK 1.6 material describes the WebView-based plugin host. OpenPlayer 2.x on Windows uses a GPUI native player; a new SDK is deferred.

- Keep [the SDK developer guide](sdk-1.6-developer-guide.md) aligned with the host bridge and the SDK types in the sibling `openplayer-plugins` repository.
- Document the actual manifest validator, permission allowlists, declared event subscriptions and host-owned redb storage. Use capability checks for optional features and distinguish experiments from released APIs.
- Prefer `sidePanel` for playlist-style views. Use a semi-transparent themed surface, expose `frameOpacitySetting` for user-tunable opacity, and reuse the host's theme tokens and classes.
- Start new subtitle integrations with `openplayer.subtitle.documents.create` and the document API. Older generated-subtitle helpers are compatibility aliases.
- Use `openplayer.artifacts.list`, `info`, `remove` and `clear` for managed clips and captures. Audio clip access requires `audio.extract`, and frame capture access requires `mpv.capture`.
- Do not invent `ai.*`, `transcription.*`, `translation.*` or provider-specific host permissions. Compose existing media, subtitle, export and task APIs or add the missing primitive.
- For host or contract changes, verify `verify:shell` and the frontend build as described in the [root guide](../../AGENTS.md). Test official examples in the plugin repository when they change.

## Subagent Workflow

Split parallel SDK work into independent lanes such as host implementation, SDK types, documentation and checks. Assign each edit a single owner and keep write ownership disjoint.

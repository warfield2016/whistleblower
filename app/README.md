# Whistleblower · Basecamp app

`type: "ui_qml"` Basecamp module. The UI is a thin QML shell that hands JSON to the
`doc-index` core module via the `logos.module("doc-index")` bridge — all orchestration
lives in [`doc-index-core`](../crates/doc-index-core).

## Files

- `metadata.json` — Basecamp manifest. Declares `dependencies: ["doc-index", "storage_module", "delivery_module"]`.
- `qml/Main.qml` — the entire UI: file picker, metadata form, Publish / Anchor / Lookup buttons, status pane.
- `icons/` — placeholder for the module icon.

## Build & install (production)

Requires the Logos Nix dev environment and the `lgpm` package manager:

```bash
nix build .#whistleblower-lgx-portable -o /tmp/whistleblower.lgx
lgpm install /tmp/whistleblower.lgx \
    --modules-dir ~/.local/share/Logos/LogosBasecamp/modules
```

Then launch Basecamp; the app appears under "Apps".

## Run in isolation (development)

For UI iteration without a full Basecamp install, the `logos-standalone-app` harness can
load a single QML module:

```bash
git clone https://github.com/logos-co/logos-standalone-app
cd logos-standalone-app
./run.sh --module ~/Python\ experiments/whistleblower/app
```

When `logos` is undefined (running outside the host), the app gracefully shows a "module
unavailable" status — you can still verify the UI shape, but Publish/Anchor calls will be
no-ops until the host injects the bridge.

## Architecture note

This module is intentionally thin. **No business logic in QML.** If the upload flow grows
more complex (e.g. client-side hashing, file chunking, encrypted previews) those features
belong in `doc-index-core` so other Basecamp apps inherit them. See
[../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for the full design.

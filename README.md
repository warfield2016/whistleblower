# Whistleblower

A reference Logos Basecamp app for censorship-resistant document publication, plus
a reusable document-indexing module that any Logos app can depend on.

> **Status:** architecture + scaffold + tests + mocks + interactive web demo. Real Logos
> backend integration (Codex / Waku / SPEL CLI) lives behind the `--features real-logos`
> and `--features real-spel` flags as stubs ready to be wired up. See
> [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for what's built and what's next.

Submitted to [LP-0017 · Whistleblower](https://github.com/logos-co/lambda-prize/blob/main/prizes/LP-0017.md).

## Try it in your browser

The reusable Rust orchestration module is compiled to WebAssembly and runs in a Next.js
page hosted on Vercel — try the upload → broadcast → anchor flow without installing
anything. See [web/](web/) for the source and [scripts/vercel-build.sh](scripts/vercel-build.sh)
for the deployment config.

## What this is

A complete worked example of the **upload → broadcast → anchor** pattern over the Logos stack:

1. **Upload** any file to [Codex](https://github.com/logos-co/logos-storage-module) (durable, content-addressed).
2. **Broadcast** an envelope on [Waku](https://github.com/logos-co/logos-delivery-module) (peer-to-peer pub/sub) so the document is immediately discoverable.
3. **Anchor** the CID on [LEZ](https://github.com/logos-blockchain/logos-execution-zone) via the chronicle-registry program — either by the publisher, or **by any third party** who heard the broadcast.

The third-party anchoring is the censorship-resistance property: the publisher never has to be online, hold tokens, or coordinate with anyone for the document to gain durable on-chain provenance.

## Components

| Path | What it is |
|---|---|
| [`crates/registry-core/`](crates/registry-core/) | Shared wire types (Instruction, Envelope, RegistryEntry). Used by both the on-chain program and off-chain clients so the wire format can't drift. |
| [`crates/doc-index-core/`](crates/doc-index-core/) | **The reusable headless module.** Compiles to `libdoc_index_core.{so,dylib,dll}` + `metadata.json` (type: "core"). Exposes JSON-in/JSON-out methods over a C ABI, consumable from QML/JS via `logos.module("doc-index")` or any FFI host. |
| [`crates/doc-index-cli/`](crates/doc-index-cli/) | `doc-index` binary — CLI wrapper for smoke-testing and demos. |
| [`crates/batch-anchor/`](crates/batch-anchor/) | `batch-anchor` daemon — permissionless third-party anchor. Subscribes to the Waku topic, accumulates CIDs, flushes to chronicle-registry in batches. SQLite-backed idempotency state. |
| [`programs/chronicle-registry/`](programs/chronicle-registry/) | The LEZ program (Rust + SPEL + RISC0). `InitRegistry` + `IndexBatch` instructions, idempotent, with the pure state-transition logic in `apply_instruction` (unit-testable without a sequencer). |
| [`app/`](app/) | The Basecamp app (`type: "ui_qml"`). Thin QML shell over the doc-index module. |
| [`web-demo/`](web-demo/) | WASM bindings for the doc-index pipeline. Compiled via wasm-pack. |
| [`web/`](web/) | Next.js app that hosts the WASM demo. Deployed to Vercel. |
| [`docs/`](docs/) | Architecture, the LEZ-vs-zone-SDK justification, CU benchmarks (forthcoming). |
| [`scripts/demo.sh`](scripts/demo.sh) | Reproducible end-to-end demo. Run from a clean clone — no manual editing required. |
| [`scripts/vercel-build.sh`](scripts/vercel-build.sh) | Installs Rust + wasm-pack and builds the web demo in the Vercel container. |

## Quickstart (mocked backends, no Logos infra needed)

```bash
# Clone
git clone <repo-url> whistleblower
cd whistleblower

# Build + test
cargo test --workspace

# End-to-end demo
./scripts/demo.sh
```

The demo runs the full pipeline against in-process mocks: publishes a file, anchors its CID, looks up the registry, runs the batch-anchor daemon for 10 seconds. **No Codex, Waku, or LEZ sequencer required.**

## Real Logos integration (in progress)

The `doc-index-core::clients::real` module (gated on `--features real-logos`) holds stubs for:

- **CodexClient** — FFI to `liblogosstorage`, calling `uploadFile(path, contentType)` and emitting CIDs.
- **WakuClient** — FFI to `liblogosdelivery`, `send(topic, payload)` / `subscribe(topic)`.
- **AnchorClient** — shells out to `lgs spel --idl ./idl/chronicle.json --program-id <hex> index_batch ...`.

The chronicle-registry program's `real-spel` feature pulls in `spel-framework` from `https://github.com/logos-co/spel` and `nssa-core` from `https://github.com/logos-blockchain/logos-execution-zone`. Build with:

```bash
# Requires the Logos dev environment (https://github.com/logos-co/scaffold)
RISC0_DEV_MODE=0 cargo build --release --features real-spel -p chronicle-registry
cargo risczero build --manifest-path methods/guest/Cargo.toml
lgs deploy
```

## Anchoring approach

We chose the **LEZ program** approach over the zone SDK alternative because:
- The zone SDK requires a "single designated actor to perform consensus inscription" (per the prize spec), which reintroduces the centralised takedown surface this project exists to eliminate.
- LEZ programs offer better tooling (SPEL, IDL generation, `lgs` CLI), composability (other programs can call us via tail calls), and event emission (LP-0012 hooks).

Full reasoning in [docs/ANCHOR_CHOICE.md](docs/ANCHOR_CHOICE.md).

## Tests

```bash
cargo test --workspace
```

41 unit + integration tests across:
- `registry-core` — wire format, metadata hashing, CID validation
- `chronicle-registry` — registry state transitions, idempotency, error codes
- `doc-index-core` — Indexer orchestration, retry, dedup, FFI roundtrip
- `batch-anchor` — SQLite idempotency state
- `doc-index-core/tests/end_to_end.rs` — full pipeline through mocks

## Repository layout

```
whistleblower/
├── Cargo.toml                      # workspace
├── README.md                       # you are here
├── docs/
│   ├── ARCHITECTURE.md             # system design + data flow
│   ├── ANCHOR_CHOICE.md            # LEZ-vs-zone-SDK justification
│   ├── API.md                      # doc-index-core public API
│   └── CU_BENCHMARKS.md            # compute unit measurements (TODO)
├── crates/
│   ├── registry-core/              # shared wire types
│   ├── doc-index-core/             # reusable module (the strategic asset)
│   │   ├── metadata.json           # Basecamp manifest (type: "core")
│   │   ├── src/lib.rs              # public API
│   │   ├── src/indexer.rs          # orchestration
│   │   ├── src/clients/mod.rs      # backend traits
│   │   ├── src/clients/mock.rs     # in-process implementations
│   │   ├── src/ffi.rs              # C ABI for QML/JS consumption
│   │   └── tests/end_to_end.rs     # integration test
│   ├── doc-index-cli/              # `doc-index` CLI binary
│   └── batch-anchor/               # `batch-anchor` daemon binary
│       └── src/state.rs            # SQLite idempotency
├── programs/
│   └── chronicle-registry/         # LEZ program (Rust + SPEL)
├── methods/guest/                  # RISC0 guest builds
├── app/
│   ├── metadata.json               # Basecamp manifest (type: "ui_qml")
│   ├── qml/Main.qml                # the UI
│   └── README.md                   # how to build + install
├── tests/integration/              # workspace-level e2e harness
├── scripts/demo.sh                 # the reproducible demo
├── flake.nix                       # Nix build entry (TODO)
└── .github/workflows/ci.yml        # CI: fmt, clippy, test, demo smoke
```

## License

MIT OR Apache-2.0.

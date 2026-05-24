# Whistleblower · Architecture

A reference Logos Basecamp app that demonstrates the **upload → broadcast → anchor** pipeline for censorship-resistant document publication, with a reusable headless module that any other Logos app can depend on.

## Thesis in one paragraph

A publisher wants their document to survive takedowns. They have two adversaries: (1) a hosting provider that can delete the bytes, and (2) an index operator that can hide the document from discovery. The Logos stack addresses (1) with Codex (content-addressed, distributed) and (2) with Waku (peer-to-peer pub/sub) — but neither alone gives a *durable, queryable record* that the document existed at time T with CID C. That record is what an on-chain LEZ registry provides. Crucially, the publisher should not need to pay on-chain fees or even be online to anchor — anyone watching the Waku topic can batch-anchor accumulated CIDs in a single transaction. This three-layer separation (storage / discovery / record) is what makes the system resistant to all three censorship attack surfaces *simultaneously*.

## Component map

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Basecamp app shell                             │
│            (Qt6 / QML, type: "ui_qml", .lgx package)                │
│                                                                     │
│   File picker → metadata form → [Publish] [Anchor on-chain]         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ logos.module("doc-index").publishFileJson(…)
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│            doc-index-core (reusable headless module)                │
│      (Rust → cbindgen → libdoc_index.{so,dylib,dll})                │
│                  type: "core", .lgx package                         │
│                                                                     │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│   │  Publisher   │  │  Broadcaster │  │  Anchor (optional, here  │  │
│   │  (→ storage) │  │  (→ delivery)│  │   or via batch CLI)      │  │
│   └──────┬───────┘  └──────┬───────┘  └──────────┬───────────────┘  │
│          │                 │                     │                  │
│          └─── upload returns CID → envelope → broadcast ─────┐      │
│                                                              ▼      │
│                                                       optional anchor│
└───────────┬───────────────────────┬──────────────────────────┬──────┘
            │                       │                          │
            ▼ (real)                ▼ (real)                   ▼ (real)
   ┌────────────────┐     ┌──────────────────┐       ┌──────────────────┐
   │  Codex via     │     │  Waku via        │       │  SPEL/LEZ via    │
   │  storage_module│     │  delivery_module │       │  sequencer JSONRPC│
   └────────────────┘     └──────────────────┘       └──────────────────┘
            │                       │                          ▲
            ▼ (mock)                ▼ (mock)                   │ submits batch
   ┌────────────────┐     ┌──────────────────┐       ┌──────────────────┐
   │ in-memory map  │     │ in-process pubsub│       │ in-memory registry│
   └────────────────┘     └──────────────────┘       └──────────────────┘

                              ┌─────────────────────────────────────┐
                              │      batch-anchor CLI (Rust bin)    │
                              │                                     │
                              │  subscribe(waku topic)              │
                              │    → accumulate (cid, hash)         │
                              │    → SQLite idempotency state       │
                              │    → batch ≥1 per N seconds         │
                              │    → submit to chronicle-registry   │
                              └─────────────────────────────────────┘
```

## Crate layout

```
whistleblower/
├── Cargo.toml                 # workspace
├── crates/
│   ├── registry-core/         # shared types: Instruction, RegistryEntry,
│   │                          # envelope schema, metadata_hash helpers.
│   │                          # no_std-compatible so the program can use it.
│   ├── doc-index-core/        # the reusable headless module.
│   │   ├── src/clients/       # CodexClient, WakuClient, AnchorClient traits
│   │   ├── src/clients/mock.rs # in-process implementations for tests + dev
│   │   ├── src/indexer.rs     # the public façade: Indexer::publish/anchor/subscribe
│   │   ├── src/ffi.rs         # cbindgen exports (publishFileJson etc.)
│   │   └── cbindgen.toml      # C header generation config
│   ├── doc-index-cli/         # CLI wrapper for the module — useful for
│   │                          # smoke-testing and the demo script
│   └── batch-anchor/          # the permissionless batch anchor CLI
│       └── src/state.rs       # SQLite idempotency state (last_anchored_at + seen_cids)
├── programs/
│   └── chronicle-registry/    # the LEZ program (SPEL)
│       └── src/lib.rs         # #[lez_program] mod with init_registry + index_batch
├── methods/guest/             # RISC0 guest builds (compiles registry to ELF)
├── app/
│   ├── metadata.json          # Basecamp manifest (type: "ui_qml")
│   └── qml/Main.qml           # minimal UI: file picker + publish + anchor
├── tests/integration/         # end-to-end Rust tests against real sequencer
├── scripts/demo.sh            # the reproducible demo script
├── docs/
│   ├── ARCHITECTURE.md        # this file
│   ├── ANCHOR_CHOICE.md       # LEZ-vs-zone-SDK justification
│   ├── API.md                 # doc-index-core public API
│   └── CU_BENCHMARKS.md       # compute unit measurements
└── .github/workflows/ci.yml
```

## Data flow, step-by-step

### Publish (immediate)

1. User selects a file in the Basecamp QML UI and fills in title/description/tags.
2. QML calls `logos.module("doc-index").publishFileJson(JSON.stringify(req))`.
3. `doc-index-core::Indexer::publish_file`:
   - reads the file bytes (caller already opened it)
   - calls `CodexClient::upload(bytes) -> Cid` with **exponential backoff retry** on transient failure
   - builds an `Envelope { cid, title, description, content_type, size_bytes, timestamp, tags? }`
   - computes `metadata_hash = "v1:" + sha256(canonical_json(envelope))`
   - calls `WakuClient::publish(topic, envelope_bytes)` with local-set deduplication keyed by CID
   - persists `PublishedRecord { cid, envelope, metadata_hash, anchored: false }` to local state
   - returns `{ ok, cid, publish_id, metadata_hash }`
4. The document is now **immediately discoverable** by any Waku subscriber on the topic.

### Anchor (optional, by publisher or third party)

Path A — **publisher anchors their own**:
1. User clicks "Anchor on-chain" in the Basecamp UI.
2. QML calls `logos.module("doc-index").anchorBatchJson(JSON.stringify({ entries: [{cid, metadata_hash, publish_id, timestamp}] }))`.
3. `Indexer::anchor_batch` calls `AnchorClient::submit_batch(entries) -> TxHash`, which under the hood calls SPEL CLI `index_batch` instruction against `chronicle-registry`.

Path B — **third party batches**:
1. `batch-anchor` daemon runs anywhere, subscribed to the Waku topic.
2. On each received envelope, it computes the same `metadata_hash`, dedupes via local SQLite (`seen_cids`), and accumulates `(cid, metadata_hash, timestamp)` tuples.
3. Every N seconds (default 30) OR when the buffer hits 50 entries, it flushes by calling `chronicle-registry::index_batch` with the batch.
4. On success, it advances `last_anchored_at` in SQLite. On network failure, the next tick retries with the same buffer (idempotency at the program level catches double-anchors).

### Query

`Indexer::lookup(cid) -> Option<RegistryEntry>` calls the sequencer's account-state RPC against the registry PDA, decoding via SPEL IDL.

## Mocking strategy

The `clients/` module defines three traits:

```rust
#[async_trait]
pub trait StorageClient { async fn upload(&self, bytes: Bytes) -> Result<Cid>; }
pub trait DeliveryClient {
    fn publish(&self, topic: &str, bytes: &[u8]) -> Result<()>;
    fn subscribe(&self, topic: &str) -> Receiver<Envelope>;
}
pub trait AnchorClient { async fn submit_batch(&self, entries: Vec<Entry>) -> Result<TxHash>; }
```

The crate ships two impls:
- `clients::mock` — in-process; storage is a `HashMap<Cid, Bytes>`, delivery is a `tokio::sync::broadcast` channel, anchor is a `Vec<RegistryEntry>` behind a `Mutex`. **No external dependencies.** Used in unit tests and the dev REPL.
- `clients::real` — FFI bridges to `liblogosstorage`, `liblogosdelivery`, and `lgs spel ... index_batch` shell-out. Compiled only with `--features real-logos`. Empty stubs in the scaffold; finish during integration phase.

Swap-in is a single line:
```rust
// dev / tests
let indexer = Indexer::new(mock::storage(), mock::delivery(), mock::anchor());
// production
let indexer = Indexer::new(real::storage(cfg), real::delivery(cfg), real::anchor(cfg));
```

## Compute unit budget plan

The prize requires CU measurement for single-CID and 50-CID batches. Our `index_batch` is designed for sublinear per-CID cost amortization:

- **Per-instruction fixed cost** (signer check, registry PDA load): ~K_fix CUs
- **Per-CID variable cost** (hash entry into registry data, emit `Anchored` event): ~K_var CUs
- **Total** for N CIDs: `K_fix + N * K_var`

We'll measure both numbers via the `tools/cycle_bench` pattern from lez-repo and plot CU/CID vs batch size in `docs/CU_BENCHMARKS.md`. Goal: demonstrate that 50-CID batches cost less per CID than 1-CID anchors by at least 10×.

## Non-goals (deliberate)

- No content moderation, blocklists, or access control — registry is permissionless.
- No full-text search; future λPrize.
- No client-side encryption — the prize spec dropped this from earlier scope discussions.
- No identity binding for publishers — anonymous by design.
- No cross-chain bridges.

## Why this design wins on the success criteria

| Criterion | How we satisfy it |
|---|---|
| Module is "reusable by other apps without depending on the Whistleblower Basecamp app itself" | `doc-index-core` is a separate `.lgx` package with `type: "core"`. The app declares `dependencies: ["doc-index"]`. |
| Batch tool is "permissionless — no coordination with the original publisher required" | `batch-anchor` only needs the Waku topic name; no shared keys or registration. |
| Batch tool is "idempotent — re-submitting an already-registered CID does not fail" | `chronicle-registry::index_batch` checks `entries.contains(cid)` before insert; duplicates skipped silently. |
| Registry "accepts batch submissions of at least 10 CIDs per transaction" | `Instruction::IndexBatch { entries: Vec<Entry> }` with default cap at 50. |
| Demo "succeeds without modification" against a clean clone | `scripts/demo.sh` spins up sequencer in standalone mode, runs both publisher and batch-anchor processes, asserts the CID lands on-chain. |
| Video "shows terminal output to confirm `RISC0_DEV_MODE=0` was active" | `demo.sh` echoes `RISC0_DEV_MODE` at the start; recording script section reminds the recorder to capture this. |

See [ANCHOR_CHOICE.md](ANCHOR_CHOICE.md) for the LEZ-program-vs-zone-SDK decision.

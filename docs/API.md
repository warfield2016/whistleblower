# doc-index-core · API reference

This document describes the public API of the **`doc-index`** Logos core module — the reusable
piece that any Basecamp app can depend on for censorship-resistant document publication.

The module exposes two equivalent surfaces:

- **Rust:** typed API via [`Indexer`](../crates/doc-index-core/src/indexer.rs) — used by `doc-index-cli`, `batch-anchor`, and integration tests.
- **C ABI / JSON-in-out:** the methods listed below, callable from QML via `logos.module("doc-index").<method>(json)` or from any FFI host. Schemas are JSON for portability across language boundaries.

The JSON API is the "production" surface — what Basecamp apps and other Logos modules will
target. The Rust API is the "embedded" surface — for Rust binaries that statically link the
crate.

---

## Lifecycle

### `doc_index_new_mock() → IndexerHandle`

Construct an Indexer wired to in-process mock backends. Returns an opaque integer handle.
Used in dev / preview / tests. Production will use `doc_index_new_real(config_json)` once
the real-backend wiring is complete.

### `doc_index_free(handle)`

Release the handle. Subsequent calls with this handle return an error.

### `doc_index_free_string(ptr)`

Free a string returned by any of the `_json` methods. **Required after every successful call.**

---

## Methods

All methods take JSON in, return JSON out. On success: `{"ok": true, ...result-specific fields...}`.
On failure: `{"ok": false, "error": "<message>"}`.

### `publishFileJson(request, bytes) → response`

Upload bytes to Codex, build an envelope, optionally broadcast on Waku.

**Request schema:**

```json
{
  "title": "string (required, non-empty)",
  "description": "string (optional, default empty)",
  "content_type": "string (required, e.g. 'application/pdf')",
  "tags": ["string", ...]  // optional, default []
  "broadcast": true          // optional, default true
}
```

**Bytes:** raw file contents (separate argument, not part of the JSON).

**Response:**

```json
{
  "ok": true,
  "cid": "zDvZRwzk...",                       // Codex CID, multibase-encoded
  "publish_id": "f47ac10b-58cc-4372-...",     // local UUID, not on-chain
  "metadata_hash": "v1:abc123...",            // anchor-ready hash
  "broadcast": true,
  "timestamp": 1716_500_000                   // unix seconds at publish time
}
```

**Behavior:**

- Storage upload retries with exponential backoff (default 5 attempts, 250 ms → 8 s cap) on transient errors. Surfaces as `ok: false` after exhaustion.
- If `broadcast: true`, dedupes per-process by CID — re-broadcasting the same CID is silently a no-op.
- The local "published records" list is updated; visible via `listPublishedJson`.

**Errors:**

- `"title must not be empty"` — caller validation
- `"content_type must not be empty"` — caller validation
- `"storage upload retried 5 times and never succeeded: ..."` — gave up
- `"delivery: ..."` — broadcast failed (the upload succeeded; you can retry the broadcast later)

---

### `anchorBatchJson(request) → response`

Submit a batch of (CID, metadata_hash) entries to chronicle-registry on LEZ.

**Request schema:**

```json
{
  "entries": [
    {"cid": "zDv...", "metadata_hash": "v1:abc..."},
    ...
  ]
}
```

**Response:**

```json
{
  "ok": true,
  "tx_hash": "0xabc...",
  "anchored_cids": ["zDv...", ...],         // newly added
  "skipped_duplicate_cids": ["zDv...", ...] // already on-chain
}
```

**Behavior:**

- Up-front validation: empty batch → error, batch > 50 entries → error, malformed CID → error.
- Per-entry on-chain lookup before submission to populate `skipped_duplicate_cids` (informational; the program is idempotent regardless).
- Local published-records list is updated: newly anchored CIDs get `anchored: true` + the tx hash.

**Errors:**

- `"batch must not be empty"`
- `"batch size N exceeds max 50"`
- `"entry has invalid-looking CID: <cid>"`
- `"malformed metadata_hash for cid <cid>: expected v1:<64-hex>"`
- `"anchor: RPC error: ..."`

---

### `lookupJson(request) → response`

Look up a CID in the on-chain registry.

**Request:** `{"cid": "zDv..."}`

**Response:**

```json
{
  "ok": true,
  "entry": {
    "cid": "zDv...",
    "metadata_hash": [byte, byte, ...],  // 32-byte array, raw bytes
    "anchor_timestamp": 1716_500_000
  } | null
}
```

`entry: null` means the CID is not in the registry.

---

### `listPublishedJson() → response`

Return all documents published through this process's Indexer instance.

**Response:**

```json
[
  {
    "publish_id": "f47ac10b-...",
    "envelope": {
      "cid": "zDv...",
      "title": "Q3 leak",
      "description": "...",
      "content_type": "application/pdf",
      "size_bytes": 12345,
      "timestamp": 1716_500_000,
      "tags": ["finance"]
    },
    "metadata_hash": "v1:abc...",
    "anchored": false,
    "anchor_tx": null
  },
  ...
]
```

This list lives in-process only. It does NOT persist across restarts and does NOT survey what other publishers have broadcast — for those, subscribe to the Waku topic directly (see `subscribe` in the Rust API).

---

## Rust API

```rust
use doc_index_core::{Indexer, PublishRequest, clients::mock};

let indexer = Indexer::new(mock::storage(), mock::delivery(), mock::anchor());

// Publish
let receipt = indexer.publish_file(&bytes, PublishRequest {
    title: "leaked memo".into(),
    description: "Q3 budget".into(),
    content_type: "application/pdf".into(),
    tags: vec!["finance".into()],
    broadcast: true,
}).await?;

// Subscribe (the batch-anchor daemon uses this)
let mut rx = indexer.subscribe().await?;
while let Some(envelope) = rx.recv().await {
    // process envelope
}

// Anchor
indexer.anchor_batch(vec![EntryRequest {
    cid: receipt.cid,
    metadata_hash: registry_core::parse_metadata_hash(&receipt.metadata_hash).unwrap(),
}]).await?;

// Lookup
let entry = indexer.lookup(&receipt.cid).await?;
```

Full rustdoc: `cargo doc --no-deps -p doc-index-core --open`.

---

## Topic & encoding

- **Default Waku content topic:** `/whistleblower/1/document-index/borsh`
- **Envelope encoding:** borsh (the project convention; deterministic; not JSON because string-escaping ambiguity).
- **Metadata hash:** sha256 of borsh-encoded envelope, prefixed `v1:` on the wire.
- **CID format:** Codex multibase strings (`zDv...` base58btc is most common).

To use a different topic (e.g. per-deployment isolation):

```rust
let indexer = Indexer::new(...).with_topic("/myapp/1/docs/borsh");
```

---

## Stability promises

- The wire format (envelope, metadata_hash, instruction) is **stable for v1**.
- The JSON method names and schemas listed here are **stable for v1**.
- The Rust API may evolve; deprecated items will be marked, removed in next major.
- Field additions to response JSON are non-breaking; consumers should ignore unknown fields.

## Non-goals

- No content moderation / blocklists. The registry is permissionless.
- No client-side encryption. Out of scope per LP-0017 spec; if needed, the caller encrypts before
  calling `publishFileJson`.
- No full-text search. The registry is keyed by CID only; richer indexing is a separate
  follow-up λPrize.
- No identity binding. Publishers are anonymous by design.

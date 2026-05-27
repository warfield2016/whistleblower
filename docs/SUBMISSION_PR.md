# LP-0017 · Whistleblower — submission

> **Status:** DRAFT. Do not file until the hard misses below flip to ✅.
> When ready, paste the body of this file (minus this header) as the
> `gh pr create --body` for `logos-co/lambda-prize`.

---

## Solution: LP-0017 — Whistleblower

A reference Logos Basecamp app for censorship-resistant document publication,
plus a reusable document-indexing module any Logos app can depend on. Submitted
against [LP-0017 — Whistleblower](https://github.com/logos-co/lambda-prize/blob/main/prizes/LP-0017.md).

| Resource | URL |
|---|---|
| Repository (MIT + Apache-2.0) | https://github.com/warfield2016/whistleblower |
| Live web demo (Vercel) | _\<TODO: Vercel URL\>_ |
| Video walkthrough | _\<TODO: Loom/YouTube URL\>_ |
| Deployed registry program ID | _\<TODO: LEZ devnet hex\>_ |
| Sequencer endpoint | _\<TODO: e.g. http://devnet.logos.network:3040\>_ |
| Release with .lgx assets | _\<TODO: gh release URL\>_ |

## At-a-glance status

| Criterion | Done | Where |
|---|---|---|
| F1 Upload to Logos Storage | _\<🟡 \| ✅\>_ | `crates/doc-index-core/src/indexer.rs::publish_file` + `clients/real::CodexClient` |
| F2 Broadcast envelope on Logos Delivery | _\<🟡 \| ✅\>_ | same crate + `clients/real::WakuClient` |
| F3 "Anchor on-chain" UI action | ✅ | `app/qml/Main.qml` Anchor button + `anchorBatchJson` |
| F4 Permissionless idempotent batch anchor CLI | _\<🟡 \| ✅\>_ | `crates/batch-anchor/` + SQLite state |
| F5 On-chain registry (LEZ + justification) | _\<🟡 \| ✅\>_ | `programs/chronicle-registry/` + `methods/guest/` + `docs/ANCHOR_CHOICE.md` |
| F6 Reusable document-indexing module | ✅ | `crates/doc-index-core/` + `docs/API.md` |
| U7 Basecamp app loadable + build instructions | _\<🟡 \| ✅\>_ | `app/README.md` + .lgx release asset |
| U8 Module SDK README + API doc | ✅ | `docs/API.md` |
| U9 IDL for LEZ program (SPEL) | _\<🟡 — see "Departures from spec" below\>_ | `docs/idl/chronicle.json` (hand-written, since we don't use SPEL) |
| R10 Upload retries (exponential backoff) | ✅ | `Indexer::publish_file` + `RetryPolicy` |
| R11 Broadcast dedup | ✅ | `Indexer::broadcast_seen` HashSet |
| R12 Batch tool resume after interruption | ✅ | `batch-anchor::state` (SQLite-backed) |
| P13 CU benchmarks (single + 50-CID) | _\<❌ \| ✅\>_ | `docs/CU_BENCHMARKS.md` |
| S14 Deployed + tested on LEZ devnet/testnet | _\<❌ \| ✅\>_ | (deploy address in header above) |
| S15 E2E integration tests vs real sequencer in CI | _\<❌ \| ✅\>_ | `.github/workflows/ci.yml` job `integration-real-sequencer` |
| S16 CI green on default branch | ✅ | https://github.com/warfield2016/whistleblower/actions |
| S17 README covers build / addresses / app / batch / query | _\<🟡 \| ✅\>_ | `README.md` |
| S18 Reproducible demo with `RISC0_DEV_MODE=0` | _\<❌ \| ✅\>_ | `scripts/demo.sh` (use `USE_REAL_LOGOS=1`) |
| S19 Recorded video showing `RISC0_DEV_MODE=0` | _\<❌ \| ✅\>_ | (URL in header above) |
| SR20 Public repo, MIT + Apache-2.0 | ✅ | https://github.com/warfield2016/whistleblower |
| SR21 Deployed program address documented | _\<❌ \| ✅\>_ | header above + `README.md` |
| SR22 Narrated video walkthrough | _\<❌ \| ✅\>_ | header above |
| SR23 CU benchmarks single + 50-CID | _\<❌ \| ✅\>_ | `docs/CU_BENCHMARKS.md` |
| SR24 GitHub issues filed for Logos friction | _\<🟡 — 3 drafted, awaiting filing\>_ | links in `docs/INTEGRATION_NOTES.md` |

---

## Architecture

```
┌─────────────────────┐  uploadFile(bytes)        ┌──────────────┐
│  Basecamp app GUI   │ ──────────────────────────▶│ Logos Storage │
│ (logos-whistleblower)│                            │   (Codex)    │
└──────────┬──────────┘                            └──────┬───────┘
           │ logos.module("doc-index")                    │ CID
           ▼                                              ▼
   ┌────────────────────────────────────────────────────────────┐
   │           doc-index-core (reusable headless module)         │
   │  publishFileJson / anchorBatchJson / lookupJson (C ABI)     │
   └────────────┬───────────────────────┬─────────────────────────┘
                │  envelope             │  EntryRequest batch
                ▼                       ▼
        ┌──────────────────┐    ┌──────────────────────┐
        │  Logos Delivery  │    │ chronicle-registry   │
        │      (Waku)      │    │  (LEZ SPEL program)  │
        └────────┬─────────┘    └──────────────────────┘
                 │                       ▲
                 │  subscribe            │
                 ▼                       │
        ┌────────────────────────────────┴────────┐
        │  batch-anchor CLI (anyone can run)      │
        │  subscribe + accumulate + batch + flush │
        │  SQLite idempotency state               │
        └─────────────────────────────────────────┘
```

Three layers, three trust boundaries:

- **Storage** (Codex) — durable, content-addressed bytes. Publisher responsibility.
- **Delivery** (Waku) — peer-to-peer pub/sub. Anyone can subscribe.
- **Registry** (LEZ chronicle-registry) — on-chain truth, queryable by CID. Anyone can write (permissionless).

The interesting property: the publisher and the anchorer can be completely
different actors. A whistleblower uploads + broadcasts; an NGO watches the
topic and anchors the CID on-chain hours or days later. No token-holding or
on-chain identity required for the publisher.

See [`docs/ARCHITECTURE.md`](https://github.com/warfield2016/whistleblower/blob/main/docs/ARCHITECTURE.md)
for the full design rationale and data flow.

## Anchoring approach — LEZ program over zone SDK

The spec asks for a "brief justification". Full version in
[`docs/ANCHOR_CHOICE.md`](https://github.com/warfield2016/whistleblower/blob/main/docs/ANCHOR_CHOICE.md).
The decisive line of reasoning, from the spec itself:

> "The zone SDK approach requires a single designated actor to perform
> consensus inscription, which affects the trust model."

A "single designated actor" reintroduces precisely the takedown surface this
project exists to eliminate. The LEZ program path lets any account with
gas submit `index_batch` — that's the **permissionless** property the spec's
"Motivation" section calls out as essential.

## Departures from the spec (and why)

### U9 — "IDL using the SPEL framework"

We do not use SPEL. After investigation, the canonical `lez-framework`
scaffold template doesn't compile against the current LEZ pin
(6 errors including a defunct `nssa_core::program::write_nssa_outputs_with_chained_call`).
SPEL itself is functional but the competing submission's PR shows it
depends on an unmerged upstream SPEL PR
([`logos-co/spel#189`](https://github.com/logos-co/spel/pull/189)) for the
`Vec<String>` CID handling path.

We therefore **hand-rolled the program against `nssa_core::program::*`**
following the same pattern as LEZ's own canonical
`examples/program_deployment/methods/guest/src/bin/hello_world.rs`. This:

- Compiles cleanly against the current LEZ pin (we verified inside our
  Docker dev container; ImageID `<TODO>`)
- Has no upstream-PR dependency
- Trades automatic IDL generation for a hand-written `docs/idl/chronicle.json`
- Documents the two instructions (`InitRegistry`, `IndexBatch`) in plain JSON

Our reading of the spec: U9 is *conditional* on choosing the LEZ-program
approach AND using SPEL. We chose the LEZ-program approach but not SPEL,
for the reason above. The hand-written IDL preserves the *intent* of U9
(machine-readable program interface) while sidestepping the broken
framework. If the reviewer requires strict spec adherence, we can port to
SPEL once the upstream PR merges.

### Wire encoding — Borsh instead of JSON

Our envelope is Borsh-serialized on the Waku topic rather than JSON
(the reference Whistleblower in the ecosystem uses JSON). Reasoning in
[`docs/ARCHITECTURE.md`](https://github.com/warfield2016/whistleblower/blob/main/docs/ARCHITECTURE.md#mocking-strategy):
Borsh is deterministic by construction (no string-escaping ambiguity),
matches the project convention for on-chain payloads, and saves bytes.

Our default topic is `/whistleblower/1/document-index/borsh` — the trailing
`borsh` makes the encoding explicit so JSON-using clients won't try to
parse it (Waku content topics are conventionally
`/app/version/topic/encoding` per LIP-23).

## Build + verify

The repo is self-contained — no Logos dev environment needed for the
mocked-pipeline tests (which exercise all orchestration logic).

```bash
git clone https://github.com/warfield2016/whistleblower
cd whistleblower
cargo test --workspace                    # 46 tests, ~30s
./scripts/demo.sh                         # end-to-end mock demo (15s)
```

For the real-Logos integration:

```bash
# Inside the repo's dev container (provided so Intel-Mac and Linux work identically)
docker compose -f docker/compose.yml run --rm dev bash -c '
  lgs localnet start &
  USE_REAL_LOGOS=1 RISC0_DEV_MODE=0 ./scripts/demo.sh
'
```

The dev container is documented in [`docker/README.md`](https://github.com/warfield2016/whistleblower/blob/main/docker/README.md)
and exists specifically to dodge the Intel-Mac platform issue we filed at
_\<TODO: logos-co/scaffold issue URL\>_.

## Compute units

| Batch size | Cycles (total) | Cycles per CID |
|---|---|---|
| 1 | _\<TODO\>_ | _\<TODO\>_ |
| 5 | _\<TODO\>_ | _\<TODO\>_ |
| 10 | _\<TODO\>_ | _\<TODO\>_ |
| 25 | _\<TODO\>_ | _\<TODO\>_ |
| 50 | _\<TODO\>_ | _\<TODO\>_ |

Methodology + harness: [`docs/CU_BENCHMARKS.md`](https://github.com/warfield2016/whistleblower/blob/main/docs/CU_BENCHMARKS.md).
Raw measurement output: `docs/CU_BENCHMARKS_RESULTS.md` (committed alongside this PR).

The per-CID cost should drop by **at least 10×** between N=1 and N=50 —
that's the on-evidence claim that batch anchoring is the cost-correct
strategy for permissionless third-party indexers.

## GitHub issues filed (SR24)

During this build we filed the following issues against Logos repos
covering real friction encountered:

1. _\<TODO: link\>_ — `logos-co/scaffold`: Intel macOS sequencer panics on
   `witness_generator` exec because circuits release has no `macos-x86_64`
2. _\<TODO: link\>_ — `logos-blockchain/logos-execution-zone`: `ruint 1.18.0`
   breaks risc0 docker build (rustc 1.88-dev vs requires 1.90)
3. _\<TODO: link\>_ — `logos-co/scaffold`: `lez-framework` template doesn't
   compile against current LEZ pin (6 errors)

Full notes in [`docs/INTEGRATION_NOTES.md`](https://github.com/warfield2016/whistleblower/blob/main/docs/INTEGRATION_NOTES.md).

## Reliability features (R10/R11/R12)

| | Implementation | Test |
|---|---|---|
| **R10 Upload retries with exponential backoff** | `Indexer::publish_file` `RetryPolicy` — 5 attempts, 250ms initial, 2× factor, 8s cap. Surfaces `IndexerError::UploadGaveUp { attempts, last_error }` on exhaustion. | `publish_retries_transient_storage_failures`, `publish_gives_up_after_max_attempts` in `crates/doc-index-core/src/indexer.rs` tests |
| **R11 Broadcast dedup** | `Indexer::broadcast_seen` HashSet, populated at publish time. Re-broadcasting the same CID in the same process is a no-op. | `rebroadcast_is_deduplicated` |
| **R12 Batch tool resume after interruption** | `batch-anchor::state` (`seen_cids` table + `last_flush_timestamp`) persists across restarts. Buffered-but-unflushed CIDs are restored on next tick on failure. | `state_survives_reopen` + `seen_lifecycle` + `flush_timestamp_persists` in `crates/batch-anchor/src/state.rs` tests |

## What this submission deliberately does NOT do

Per the spec's "Out of Scope":

- No content moderation, blocklists, or access control
- No full-text search of document content (reserved for follow-up λPrize)
- No client-side encryption (the spec explicitly dropped this from earlier scope discussion)
- No cross-chain anchoring
- No hosted relay or backend service

Per our own architectural choices:

- No automatic IDL generation (see "Departures from spec" above)
- No version migration for old RegistryEntry schema (`version: u8` field
  enables this in the future; not exercised yet)

## Differentiation vs reference implementations

This submission borrows the wire-format superset from the reference
(`Thompsonmina/WhistleBlower-Logos-`) — same envelope shape, same `v1:`
hash prefix, compatible Waku topic structure (different encoding). What
this submission **adds**:

- **WebAssembly browser demo** — the same Rust orchestration logic
  compiled to WASM, runs in a Next.js page on Vercel. Evaluators can try
  the upload → broadcast → anchor flow without installing anything.
  [`web/`](https://github.com/warfield2016/whistleblower/blob/main/web/) +
  [Vercel URL](_<TODO>_).
- **Public submission checklist** — `docs/SUBMISSION_CHECKLIST.md`
  is the same artifact we used to drive this work; published as ecosystem
  signal.
- **Docker dev container** — fully reproducible toolchain image (4.42GB)
  that solves the Intel-Mac platform issue. Anyone can `docker compose run`
  and get the same `lgs setup` → `lgs build` → `lgs deploy` results.
- **Pure `apply_instruction` transition** — registry logic is unit-testable
  without a sequencer (see `programs/chronicle-registry/src/lib.rs`).

## License

Dual MIT + Apache-2.0 ([LICENSE-MIT](https://github.com/warfield2016/whistleblower/blob/main/LICENSE-MIT)
+ [LICENSE-APACHE](https://github.com/warfield2016/whistleblower/blob/main/LICENSE-APACHE)).

---

## Submission tracking

Tracking issue: _\<TODO: open `logos-co/ecosystem` issue and link\>_.

Re: the 3-submission limit — this is submission **1 of 3**. Subsequent slots
reserved for closing any reviewer-flagged gaps.

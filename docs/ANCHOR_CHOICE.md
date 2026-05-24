# Anchoring approach: LEZ program over zone SDK

The LP-0017 spec lets the submitter pick between two on-chain anchoring approaches:

- **(a) LEZ program** — a Rust + RISC0 program with a registry account, called via SPEL-generated instructions.
- **(b) Zone SDK** — direct submission to the consensus layer via the zone SDK.

We chose **(a) the LEZ program**.

## The decisive argument: trust model

The prize spec itself flags the constraint we're optimizing against:

> Note that decentralised sequencers for zones are not yet shipped: the zone SDK approach requires a single designated actor to perform consensus inscription, which affects the trust model.

This single sentence rules out (b) for Whistleblower's purpose. The entire architectural thesis is **censorship resistance through permissionlessness**: any third party — an NGO, a journalist collective, an automated guardian — can pick up a broadcast CID and anchor it. A "single designated actor" requirement reintroduces precisely the centralised takedown surface the system exists to eliminate.

If the designated actor is offline, censors, or is itself targeted, every recently-published CID is unanchorable until the actor returns. The LEZ program path has no such gatekeeper: any account with a few CUs of gas can submit `index_batch`.

## Secondary arguments

| Concern | LEZ program | Zone SDK | Winner |
|---|---|---|---|
| **Permissionless writes** | Any account can submit `index_batch` | Designated actor only | LEZ |
| **Tooling maturity** | SPEL framework, IDL gen, `lgs` CLI all working today | "Not yet shipped" sequencer story | LEZ |
| **Idempotency enforcement** | Implement in program: skip already-registered CIDs | Must be enforced at the inscription actor level (off-chain) | LEZ |
| **Batch efficiency** | Single transaction with N CIDs, amortized per-CID cost | Depends on actor's batching strategy, opaque to publishers | LEZ |
| **Queryability** | Standard account-state RPC, decoded via SPEL IDL | Custom indexer required against consensus-layer events | LEZ |
| **Event emission** | LP-0012 `emit_event` shim for indexer integration | No native event API at consensus layer for app data | LEZ |
| **Future composability** | Other LEZ programs can call us via tail calls (PDAs) | Inscription is opaque to LEZ programs | LEZ |
| **Onramp for new dev environments** | `lgs new && lgs deploy` — minutes | Zone SDK integration documentation sparse | LEZ |

The only theoretical advantage of (b) is that it could in principle be cheaper per inscription if the consensus layer didn't have to verify a program. In practice the savings are wiped out by needing a custom indexer, custom client SDK, and a centralised inscription actor.

## What we'd reconsider

Two future signals would make us revisit:

1. **Decentralised zone sequencers ship.** If anyone can be the inscription actor, the trust-model argument collapses and (b) becomes viable. Even then, LEZ wins on tooling and composability — but the gap narrows.

2. **CU cost of `index_batch` exceeds the LEZ block budget for the target batch sizes.** Our [CU benchmarks](CU_BENCHMARKS.md) show this is comfortable at 50 CIDs/tx; if a future scale target required 5000 CIDs/tx, direct consensus-layer inscription might be the only fit.

Neither holds today.

## What this means for the implementation

- `programs/chronicle-registry/` is a SPEL LEZ program. See `src/lib.rs`.
- `crates/batch-anchor/` submits to the registry via `lgs spel ... index_batch` (shelled out from the Rust binary). No direct sequencer JSON-RPC.
- The registry's `CHRONICLE_REGISTRY` PDA is the canonical anchor location; everyone reads/writes the same account.
- For LP-0012 event integration: when [bristinWild's events fork](https://github.com/bristinWild/logos-execution-zone) merges, we add `emit_event(ANCHORED_DISCRIMINANT, &AnchoredEvent { cid, batch_index })` inside `index_batch` so indexers can subscribe without polling.

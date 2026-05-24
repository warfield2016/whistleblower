# Compute unit benchmarks

The prize requires CU measurement for single-CID and 50-CID batch `index_batch` calls.
This document captures the methodology and (forthcoming) results.

## Status

**Pending real-LEZ integration.** The methodology and harness are designed; the numbers
will be populated once `chronicle-registry` is built against the SPEL framework and run
through `lez-repo/tools/cycle_bench`.

## Methodology

We piggyback on the existing `tools/cycle_bench` Criterion harness from
[`logos-blockchain/logos-execution-zone`](https://github.com/logos-blockchain/logos-execution-zone)
which already produces CU measurements for token / vault / amm programs. Adding chronicle-registry
to this harness is a 1-file change:

```rust
// In lez-repo/cycle_bench/benches/chronicle_registry.rs (added during real-LEZ wire-up):
use chronicle_registry::{apply_instruction, RegistryState};
use registry_core::{Instruction, EntryRequest};

fn bench_index_single(c: &mut Criterion) {
    c.bench_function("chronicle_registry::index_batch::1", |b| {
        b.iter_batched(
            || initialized_state(),
            |state| {
                apply_instruction(
                    state,
                    Instruction::IndexBatch { entries: vec![entry()] },
                    0,
                ).unwrap()
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_index_batch_50(c: &mut Criterion) {
    c.bench_function("chronicle_registry::index_batch::50", |b| {
        b.iter_batched(
            || initialized_state(),
            |state| {
                apply_instruction(
                    state,
                    Instruction::IndexBatch { entries: (0..50).map(|_| entry()).collect() },
                    0,
                ).unwrap()
            },
            BatchSize::SmallInput,
        );
    });
}
```

For full on-chain measurements (including SPEL framework + nssa overhead, which our pure
`apply_instruction` benchmark omits), we run the same instructions against a standalone
sequencer in production proof mode:

```bash
RISC0_DEV_MODE=0 just run-sequencer        # in one terminal
cargo run --bin cu_measure -- chronicle_registry index_batch --entries 1
cargo run --bin cu_measure -- chronicle_registry index_batch --entries 50
```

The `cu_measure` runner reads the sequencer's tx receipt (per LP-0012's added
`get_transaction_receipt` RPC) and prints the `cycles_used` field from the RISC0 session info.

## Expected shape

`index_batch`'s cost decomposes into:

- **Fixed cost per call (K_fix):** PDA lookup, account deserialization, instruction parse, account post-state write, signer verification. Independent of batch size.
- **Per-CID variable cost (K_var):** `contains()` linear scan over existing entries (current implementation; logarithmic via a sorted Vec if needed later), hash entry into Vec, eventual event emission (LP-0012).

**Total for N entries:** `K_fix + N * K_var`.

**Per-CID cost:** `K_fix/N + K_var`, which is minimised by large N.

The expected per-CID cost ratio for N=1 vs N=50 is roughly `(K_fix + K_var) / (K_fix/50 + K_var)`.
With K_fix typically dominant in small programs (account I/O is expensive), this ratio should
land **≥ 10× cheaper per CID at N=50**, satisfying the prize's batch-efficiency expectation.

## Once measured: target chart

A reproducible chart in `docs/CU_BENCHMARKS.md` should show:

```
batch size N    total cycles    cycles per CID
─────────────   ─────────────   ──────────────
       1         ~K_fix + K_var
       5         …
      10         …
      25         …
      50         …
```

with the per-CID column dropping monotonically — that's the on-evidence claim that batch
anchoring is the cost-correct strategy for permissionless third-party indexers.

## Linear scan caveat

The current `RegistryState::contains` is O(N) over the existing entries. At small registry sizes
(< 10k entries) this is irrelevant. Above that, the per-anchor cost begins to grow linearly with
total registry size, not just batch size — eventually making each anchor more expensive than the
previous. Two ways out:

1. **Sorted Vec + binary search** — O(log N) lookup, O(N) insertion (still acceptable for the
   anchor workload).
2. **Split state into shards keyed by CID hash prefix** — O(1) amortised lookup, but adds
   accounts and increases per-instruction CU overhead.

We defer this optimisation until benchmarks demonstrate it's needed. Captured here so future
maintainers (or evaluators) can see the bound was considered.

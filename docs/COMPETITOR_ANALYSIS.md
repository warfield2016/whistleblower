# Competitor analysis — `Thompsonmina/WhistleBlower-Logos-`

Snapshot of the competing LP-0017 submission, gathered 2026-05-25 via direct clone of
their public repo + `gh` queries against `logos-co/lambda-prize#48` and
`logos-co/ecosystem#120`. Refresh before any submission decision.

## Status at snapshot time

| Signal | Value |
|---|---|
| PR opened | 2026-05-12 |
| Last code commit (theirs) | 2026-05-15 |
| Reviewer assigned | 2026-05-21 — `fryorcraken`: "@weboko to continue review of submission." |
| Reviewer first feedback | 2026-05-25 — `weboko`: "I left some comments... will try to play with your submitted software" |
| Reviewer concrete blockers | 2026-05-26 — `weboko`: 3 blockers (see below) |
| Latest activity | 2026-05-27 — `Thompsonmina` responded; review back-and-forth ongoing |
| Validation status | ⚠️ Non-blocking warning: missing `module.json` (still unresolved) |
| PR mergeable | Yes |
| Author | Thompsonmina (`Thompson`) |

**Read:** Review is **actively in progress** as of 2026-05-27. Three concrete
blockers raised; some require upstream changes outside Thompson's control.

### Reviewer's 3 blockers (2026-05-26) and Thompson's defense (2026-05-27)

| # | weboko's blocker | Thompson's defense | Our takeaway |
|---|---|---|---|
| 1 | Submission depends on a **custom SPEL fork pin** — wants upstream SPEL for submission | "Upstream SPEL can't invoke `Vec<String>` args — that's what my [logos-co/spel#189](https://github.com/logos-co/spel/pull/189) solves." | They're **blocked on an upstream merge** that's outside Thompson's control. |
| 2 | `batch-anchor.toml` has **hardcoded `program_id`** that didn't match weboko's local deploy | "It's deterministic via `make build` (risc0 reproducible builds)." But weboko's deploy still didn't match. | This is a **clean-clone reproducibility issue** — exactly the failure mode that phase 7.1 of our checklist defends against. |
| 3 | "Mismatch with how CIDs are passed to spel — would fail if more than one passed" | "Fixed in latest push: switched from CSV format to repeated flag syntax." | Real bug that was in their submission until 2026-05-27. |

**Strategic implication:** Even if Thompson addresses issues 2 + 3 quickly, issue 1
(upstream SPEL PR merge) is a hard dependency on a third party. Our submission
should:

- **Use `lez-framework`, not SPEL.** Completely sidesteps issues 1 and 3 by
  not depending on the spel CLI's argument parsing at all.
- **Pass `program_id` via env var or CLI arg**, never hardcoded. Phase 7.1
  fresh-clone test catches this before submission.
- **Keep our window monitoring active.** Review is moving — we may have
  weeks, but we shouldn't assume months.

**Read:** Window is still real but narrower than the prior "6 days stalled"
assessment. Estimate: 1-2 weeks before this PR resolves one way or the other.

## Repo shape

```
WhistleBlower-Logos-/
├── README.md                          # extensive — has LP-17 requirement-map table
├── README_CHRONICLE_REGISTRY.md       # separate doc for the on-chain side
├── Makefile                           # build/idl/cli/deploy/setup targets
├── Cargo.toml                         # workspace
├── flake.nix + flake.lock             # nix-locked deps, including the .lgx packaging
├── scaffold.toml + spel.toml          # lgs scaffold + spel CLI configs
├── integration-test.toml              # topic + program_id contract for IT smokes
├── chronicle-registry-idl.json        # generated IDL, committed
├── demo.sh
│
├── methods/                           # RISC0 guest binary (build.rs = risc0_build::embed_methods)
├── chronicle_registry_core/           # shared types (CidRecord, Registry, error codes)
├── examples/                          # auto-generated chronicle_registry_cli + generate_idl bins
├── ffi/                               # separate C ABI cdylib crate
├── batch-anchor/                      # anchor daemon with `node up`, `init`, `watch` subcommands
├── logos-chronicle/                   # reusable module (type: "core"), vendored ffi .so
├── logos-whistleblower/               # Basecamp app (type: "ui_qml")
└── scripts/                           # setup.sh, run-app.sh, ci-local.sh, list-registry.sh
```

## Wire-format differences (ours vs theirs)

| Field | Ours (`registry_core::RegistryEntry`) | Theirs (`chronicle_registry_core::CidRecord`) |
|---|---|---|
| Storage layout | `Vec<RegistryEntry>` | `HashMap<String, CidRecord>` keyed on CID |
| `cid` field | Yes, in the record | No — used as the map key |
| `metadata_hash` | `[u8; 32]` | `[u8; 32]` ✓ same |
| `anchor_timestamp` | `u64` | `i64` (theirs allows negatives — probably defensive) |
| `anchored_by` | ❌ missing | `[u8; 32]` — who submitted this anchor |
| `version` | ❌ missing | `u8` — envelope schema version |
| Lookup cost | O(N) linear scan | O(1) hash |
| Batch size cap | 50 (matches ours) | 50 (matches ours) |
| Error code numbering | 1001..1006 | 1..8 |

| Topic / encoding | Ours | Theirs |
|---|---|---|
| Waku content topic | `/whistleblower/1/document-index/borsh` | `/chronicle/1/document-index/json` |
| Envelope encoding | Borsh | JSON |
| Registry PDA seed | `[literal("chronicle_registry")]` (planned) | `[b"registry"]` |

**Interop note:** the topics differ so we never see each other's traffic. There's no
interop conflict — each project runs its own registry on its own topic. The choice
of borsh vs JSON for the envelope is a design preference; ours saves bytes, theirs
is human-readable in `nwaku` debugger output.

## Their architecture wins (worth borrowing)

1. **`HashMap<String, CidRecord>` registry** — O(1) lookup, deterministic serialization
   via borsh (which sorts keys when serializing maps). Cleanly beats our `Vec`.
2. **`anchored_by` + `version` fields** — forward-compatible audit + schema migration.
3. **Separate `ffi/` crate** — cleanly isolates the C ABI surface from the orchestration
   logic in `logos-chronicle`. The Makefile vendor target copies the `.so` into
   `logos-chronicle/vendored/` so the flake-built `.lgx` package picks it up.
4. **Makefile with `-include $(STATE_FILE)` + `define save_var`** — persists the
   signer ID across Make invocations without a database. Sharp Make-fu.
5. **`risc0_build::embed_methods()` in `methods/build.rs`** — canonical RISC0 build
   pattern, generates the `*_ELF` / `*_ID` constants used by the SPEL macro.
6. **Topic isolation between prod and integration test** — separate
   `batch-anchor.toml` and `batch-anchor.it.toml`, only `content_topic` differs.
   Lets the demo recording run against a real sequencer without polluting any
   shared topic.
7. **`scripts/setup.sh` is exemplary** — idempotent, probes for sequencer port
   before starting it, handles signer minting + toml rewriting, includes
   inline numbered docs in the header.
8. **Demo video already uploaded** as a GitHub release asset
   (`releases/download/demo-v1/demo.mp4`) — referenced from README row S19.
9. **`spel.toml` + `scaffold.toml`** at repo root — config-as-data for the SPEL
   CLI and `lgs` scaffold.

## Their gaps (our differentiation surface)

| Gap | Confirmed by | Our advantage |
|---|---|---|
| **No web demo** | Inspected entire repo, no `web/` or WASM dir | Vercel WASM demo, guided tour |
| **CI integration tests `pending`** | Their own README row S15: "_pending_" | Our `.github/workflows/ci.yml` runs `cargo test --workspace` (44 tests) green |
| **No `module.json`** | GitHub Action validation warning on their PR | Add `module.json` symlinks to clear our submission warning |
| **No planning artifact** | No SUBMISSION_CHECKLIST.md or similar | Our public checklist signals ecosystem-engagement quality |
| **No `INTEGRATION_NOTES.md`-style doc** | No filed GitHub issues against logos-co repos | Phase 7.4 of our checklist plans 2-3 issues |
| **One README doing too much** | Their main README is 308 lines + has bleed across responsibilities | Our split (README + ARCHITECTURE + ANCHOR_CHOICE + API + DEPLOY + RECORDING_SCRIPT + CU_BENCHMARKS + SUBMISSION_CHECKLIST) is more navigable |

## Specific patterns to adopt verbatim

These are zero-controversy borrowings — they've been validated by the competitor
shipping them and we should match:

```makefile
# pattern 1: state persistence across Make invocations
STATE_FILE := .chronicle_registry-state
-include $(STATE_FILE)

define save_var
	@grep -v '^$(1)=' $(STATE_FILE) 2>/dev/null > $(STATE_FILE).tmp || true
	@echo '$(1)=$(2)' >> $(STATE_FILE).tmp
	@mv $(STATE_FILE).tmp $(STATE_FILE)
endef
```

```rust
// pattern 2: methods/build.rs — canonical RISC0 guest build
fn main() {
    risc0_build::embed_methods();
}
```

```bash
# pattern 3: setup.sh probe-port-before-start (avoids spawning duplicate sequencer)
if ! (exec 3<>/dev/tcp/127.0.0.1/3040) 2>/dev/null; then
    lgs localnet start
else
    exec 3<&-
fi
```

```toml
# pattern 4: separate prod and integration-test configs, topic-only difference
# batch-anchor.toml
content_topic = "/whistleblower/1/document-index/borsh"
# batch-anchor.it.toml
content_topic = "/whistleblower/it/document-index/borsh"
```

## Wire-format upgrades to consider before submission

If we adopt their forward-compat additions, our `RegistryEntry` becomes:

```rust
pub struct RegistryEntry {
    pub metadata_hash: [u8; METADATA_HASH_LEN],
    pub anchor_timestamp: i64,           // was u64 — switch to match their i64
    pub anchored_by: [u8; 32],            // NEW — who submitted the anchor
    pub version: u8,                      // NEW — envelope schema version
}

pub struct Registry {
    pub initialized: bool,
    pub entries: HashMap<String, RegistryEntry>,  // was Vec<RegistryEntry>
}
```

**Cost:** changes the on-chain layout. If we've already deployed, this is a hard
migration. Since we haven't deployed yet, the cost is just updating tests and the
program transition function. Worth doing.

## Strategic recommendation

1. **Don't engage in a feature war.** They have more deployed pieces but the
   reviewer isn't scoring features against features — they're scoring against the
   spec's success criteria. We should match every spec criterion and add
   differentiation beyond it (web demo, CI tests).
2. **Race-condition awareness.** The reviewer could come back any day and
   approve their PR. Every day we delay submission, prior expected EV drops.
3. **Strongest single move:** ship phases 1-7 of `SUBMISSION_CHECKLIST.md`
   within ~7 calendar days. That's faster than the reviewer is currently moving.
4. **Cheap defensive moves (do today):**
   - Add `module.json` to both module dirs (clear validation warning)
   - Upgrade wire format to match theirs (HashMap + anchored_by + version + i64
     timestamp) before any real deploy
   - Add a Makefile mirroring theirs
   - Add `methods/build.rs` with `risc0_build::embed_methods()`
5. **Differentiation cards we already hold:**
   - WASM web demo on Vercel
   - 44-test CI green (vs their S15 pending)
   - Public SUBMISSION_CHECKLIST.md
   - Cleaner docs split

## Action items derived from this analysis

| # | Action | Phase in checklist | Why |
|---|---|---|---|
| A1 | Add `module.json` (symlink or copy of `metadata.json`) to `app/` and `crates/doc-index-core/` | 0 (do today) | Clear the GitHub Action validation warning that their PR has |
| A2 | Upgrade `RegistryEntry` to add `anchored_by`, `version`, switch to HashMap, i64 timestamp | 2 | Wire-format parity with the competitor's superset; forward-compat |
| A3 | Add root `Makefile` with build/idl/cli/deploy/setup/status targets | 2 | Match their tooling polish; standard Logos pattern |
| A4 | Write `methods/build.rs` with `risc0_build::embed_methods()` | 2 | Canonical RISC0 pattern, required for SPEL to find the guest |
| A5 | Write `scripts/setup.sh` that mirrors their idempotent bootstrap | 2 | Required for the "clone-and-run" evaluator test |
| A6 | Split `batch-anchor.toml` and `batch-anchor.it.toml` for topic isolation | 4 | Prevents the demo recording from polluting any shared topic |
| A7 | Open an issue against `logos-co/lambda-prize` or `logos-co/ecosystem` if the validation Action checks `module.json` but the docs say `metadata.json` | 7.4 | Useful ecosystem-engagement filing |

## Re-check before submission

Re-run the status check (`gh pr view 48 --repo logos-co/lambda-prize`) before
filing our PR. If it's been merged or closed-as-approved between snapshot time
and submission, pivot per the "Aborted-sprint contingencies" section of
[SUBMISSION_CHECKLIST.md](SUBMISSION_CHECKLIST.md).

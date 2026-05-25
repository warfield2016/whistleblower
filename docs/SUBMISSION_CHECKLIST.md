# LP-0017 Submission Checklist

Step-by-step plan to take the current scaffold from "works on mocks" to "submitted PR that
wins the prize". Estimated total: **28-42 hours of focused work** across 6-8 calendar days.

Each step has:
- **Time** estimate
- **Acceptance** — how you verify it's done
- **Risk** flags where the spec or Logos tooling has known traps
- **Decision gate** at the end of each phase — when to abort or pivot

The competing submission is `Thompsonmina/WhistleBlower-Logos-` (PR
[`logos-co/lambda-prize#48`](https://github.com/logos-co/lambda-prize/pull/48), tracking
issue [`logos-co/ecosystem#120`](https://github.com/logos-co/ecosystem/issues/120),
reviewer: `weboko`). **Check its status before starting each phase.**

---

## Phase 0 — Pre-flight (today, ~1 hour)

Goal: ensure the current artifacts are as visible and polished as they can be before
investing in real-Logos integration. Cheap, high-leverage.

- [x] **0.1a** Competitor analysis. See [`COMPETITOR_ANALYSIS.md`](COMPETITOR_ANALYSIS.md)
  for the full intel snapshot. Status at 2026-05-25: PR open since May 12, reviewer
  promised feedback May 19 (6 days ago, no follow-up). Window is real.
- [x] **0.1b** Defensive parity moves from competitor analysis:
  - [x] `module.json` files added to `app/` and `crates/doc-index-core/` (clears the
    GitHub Action validation warning their PR triggered)
  - [x] Root `Makefile` added with `build`, `idl`, `deploy`, `setup`, `cli`, `test`,
    `fmt`, `clippy`, `ci`, `web`, `demo`, `status`, `clean` targets — matches their
    tooling polish
- [ ] **0.1c** Wire-format upgrade (still pending — adopt before phase 2 deploy):
  - Add `anchored_by: [u8; 32]` and `version: u8` to `RegistryEntry`
  - Switch `Registry` from `Vec<RegistryEntry>` to `HashMap<String, RegistryEntry>` for O(1) lookup
  - Switch `anchor_timestamp` from `u64` to `i64`
  - Acceptance: `cargo test -p registry-core -p chronicle-registry` passes after changes
- [ ] **0.2** Vercel deploy. Follow [DEPLOY.md](DEPLOY.md). Wait ~5-7 min for first build.
  - Acceptance: live URL loads, "Run guided tour" button works end-to-end, lookup
    panel populates after the tour
  - ⚠️ If the Vercel build fails: most likely cause is `scripts/vercel-build.sh`
    failing to install Rust. Inspect the Vercel build log; the script's first line
    is `set -euo pipefail` so any failure aborts immediately with a clear line number
- [ ] **0.3** Update root `README.md` to replace the two `_[pending]_` placeholders at
  the top with the actual Vercel URL (recording URL still pending until phase 6)
- [ ] **0.4** Record a 15-second screen capture of just the "Run guided tour" running
  on the live Vercel URL — this is a *placeholder* demo for the README until the
  full submission-quality recording in phase 6
  - Acceptance: GIF or short MP4 embedded in README top section
- [ ] **0.5** Re-check competitor PR status:
  ```bash
  gh pr view 48 --repo logos-co/lambda-prize --json state,updatedAt,reviews,comments
  gh issue view 120 --repo logos-co/ecosystem --json state,updatedAt,comments
  ```
  - If `state=MERGED` on PR #48 → STOP. Pivot to follow-up λPrize work (see "Aborted
    sprint contingencies" at the bottom of this doc).
  - If `updatedAt` is older than 14 days and no recent comments → window likely open;
    proceed.

🎯 **Phase 0 decision gate:** Is the competing PR still in unresolved review, AND did
the Vercel build succeed? If both **yes** → continue. If either **no** → re-evaluate.

---

## Phase 1 — Logos dev environment (Day 1, 2-4 hours)

Goal: install the toolchain end-to-end and confirm a known-working LEZ program (`hello_world`
from the LEZ tutorial) builds and deploys. This is the **feasibility gate** — most of
the integration risk lives here.

- [ ] **1.1** Install Rust toolchain (if not already):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup default stable
  ```
- [ ] **1.2** Install [risc0 zkVM toolchain](https://dev.risczero.com/api/zkvm/install):
  ```bash
  curl -L https://risczero.com/install | bash
  rzup install
  rzup install rust         # installs the risc0 fork of rustc + cargo subcommand
  ```
  - Acceptance: `cargo risczero --version` prints v1.x or v3.x
- [ ] **1.3** Install `lgs` scaffold CLI:
  ```bash
  git clone https://github.com/logos-co/scaffold
  cd scaffold && cargo install --path .
  lgs --version
  ```
  - Acceptance: `lgs` binary is in `~/.cargo/bin/` and prints its version
- [ ] **1.4** Download `logos-blockchain-circuits` (the proving keys):
  ```bash
  # Per scaffold README; URL/version may have updated by submission time.
  curl -L <release-url> -o circuits.tar.gz
  mkdir -p ~/.logos-blockchain-circuits && tar -xzf circuits.tar.gz -C ~/.logos-blockchain-circuits
  export LOGOS_BLOCKCHAIN_CIRCUITS=~/.logos-blockchain-circuits
  ```
  - ⚠️ This is typically a multi-GB download. Budget bandwidth and disk.
- [ ] **1.5** Verify hello-world tutorial works end-to-end:
  ```bash
  cd scaffold && lgs new hello-test && cd hello-test
  lgs setup && lgs localnet start &      # background
  lgs build && lgs deploy
  # Expect: program_id: <64-char-hex>
  ```
  - Acceptance: a program is deployed and `lgs spel inspect <program_id>` returns its
    image ID
- [ ] **1.6** Stop the localnet (`pkill lgs` or kill the bg job)
- [ ] **1.7** Document anything that broke or surprised you in
  [`docs/INTEGRATION_NOTES.md`](INTEGRATION_NOTES.md) (create this file as you go) —
  this is also the source material for the GitHub issues you'll file in phase 7

🎯 **Phase 1 decision gate:** Did hello-world fully deploy with no manual workarounds?
- **Yes** → continue, the integration is feasible.
- **No, but I worked around it** → continue, but budget +50% on phase 2 time estimates.
- **No, and the workaround is unknown** → STOP. File a Logos GitHub issue, ping in
  their Discord, and re-evaluate. The economics don't support unbounded debugging.

---

## Phase 2 — Complete the chronicle-registry SPEL program (Day 1-2, 4-6 hours)

Goal: turn the commented-out SPEL block in `programs/chronicle-registry/src/lib.rs`
into a real, compilable, deployable LEZ program.

- [ ] **2.1** Add SPEL framework dependencies to
  `programs/chronicle-registry/Cargo.toml`:
  ```toml
  [dependencies]
  spel-framework = { git = "https://github.com/logos-co/spel", rev = "<pin-this>" }
  nssa-core = { git = "https://github.com/logos-blockchain/logos-execution-zone", tag = "v0.1.0" }
  risc0-zkvm = { version = "3.0.5", default-features = false, features = ["std"] }
  ```
  - ⚠️ Pin both git revisions. SPEL and LEZ both move fast and `main` may break you.
- [ ] **2.2** Uncomment the `#[lez_program] mod chronicle_registry` block. Fill in:
  - `init_registry(registry, anchorer)` — calls `apply_instruction(state,
    InitRegistry, now())` and writes the new state back to `registry.account.data`
  - `index_batch(registry, anchorer, entries_borsh)` — deserializes
    `Vec<EntryRequest>`, calls `apply_instruction`, writes state
  - `to_spel_error(TransitionError)` mapper using `error_codes` from registry-core
  - `read_state` / `write_state` helpers
  - `now()` — use the block context timestamp if LEZ exposes it, else 0
- [ ] **2.3** Create the guest entry point at
  `methods/guest/src/bin/chronicle_registry.rs`:
  ```rust
  #![no_main]
  use chronicle_registry::__main as main;
  risc0_zkvm::guest::entry!(main);
  ```
  (or whatever pattern the lez-multisig reference uses)
- [ ] **2.4** Create `methods/guest/Cargo.toml` with the guest workspace setup — mirror
  the pattern in `lez-events/lez-repo/program_methods/guest/Cargo.toml`
- [ ] **2.5** Create the IDL generator at
  `programs/chronicle-registry/examples/src/bin/generate_idl.rs`:
  ```rust
  fn main() {
      spel_framework::generate_idl!("../../src/lib.rs", "chronicle_registry");
      // prints JSON IDL to stdout; we redirect this to docs/idl/chronicle.json
  }
  ```
- [ ] **2.6** Build the guest ELF:
  ```bash
  cargo risczero build --manifest-path methods/guest/Cargo.toml
  ls target/riscv32im-risc0-zkvm-elf/docker/  # expect chronicle_registry.bin
  ```
  - Acceptance: a `.bin` file appears
- [ ] **2.7** Generate the IDL and commit it:
  ```bash
  cargo run -p chronicle-registry --example generate_idl > docs/idl/chronicle.json
  git add docs/idl/chronicle.json
  ```
- [ ] **2.8** Update `crates/registry-core/src/lib.rs` tests with assertions that the
  IDL matches the on-chain expected types (defensive — catches drift)

🎯 **Phase 2 decision gate:** Does the guest ELF build and the IDL generate? If either
fails repeatedly, the SPEL framework version is likely incompatible. File a GitHub issue
and try a known-good pin (e.g., the rev that lez-multisig pins to).

---

## Phase 3 — Real backend clients (Day 2-3, 8-12 hours)

Goal: fill in the three stubs in `crates/doc-index-core/src/clients/real.rs` so they
actually talk to Codex, Waku, and the deployed SPEL program.

- [ ] **3.1** Add the `real-logos` feature flag dependencies to
  `crates/doc-index-core/Cargo.toml`:
  ```toml
  [features]
  real-logos = ["dep:libloading", "dep:tokio-process"]
  ```
- [ ] **3.2** Implement `CodexClient` in `real.rs`:
  - Load `liblogosstorage.{dylib,so,dll}` via `libloading`
  - Marshal `upload(bytes)` → CID string
  - Use `tokio::task::spawn_blocking` if the FFI call is sync
  - Implement exponential-backoff retry per the prize spec (already in
    `Indexer::publish_file`; just make sure `StorageError::Transient` is what gets
    returned on network failures)
  - Acceptance: `cargo test --features real-logos -p doc-index-core
    real_codex_smoke -- --ignored` uploads a file to a running Codex node and
    returns a CID
- [ ] **3.3** Implement `WakuClient` in `real.rs`:
  - Load `liblogosdelivery.{dylib,so,dll}` via `libloading`
  - `publish(topic, envelope)` → marshal borsh-encoded envelope to a Waku message
  - `subscribe(topic)` → spawn a thread that pumps Waku's message-received signal
    into a `tokio::sync::mpsc::UnboundedSender`
  - Use the topic from `registry_core::DEFAULT_WAKU_TOPIC` so the production module
    and demo broadcast on the same channel
  - Acceptance: two processes can publish/subscribe through Waku and the envelope
    round-trips
- [ ] **3.4** Implement `LgsAnchorClient` in `real.rs`:
  - Shell out to `lgs spel --idl <idl> --program-id <hex> -p <bin> index_batch
    --cids ... --metadata-hashes ...` via `tokio::process::Command`
  - Parse the JSON output to extract `tx_hash`
  - For `lookup`: shell out to `lgs spel inspect <registry-pda> --idl <idl>` and
    parse the registry state to find the CID
  - Acceptance: `submit_batch(vec![single_entry])` returns a real tx hash from
    the local sequencer
- [ ] **3.5** Update the `batch-anchor` binary's `main()` to switch on a CLI flag
  between mock and real backends:
  ```rust
  let indexer = if cli.real_logos {
      Arc::new(Indexer::new(real::codex(cfg), real::waku(cfg), real::anchor(cfg)))
  } else {
      Arc::new(Indexer::new(mock::storage(), mock::delivery(), mock::anchor()))
  };
  ```
- [ ] **3.6** Update `doc-index-cli` the same way

🎯 **Phase 3 decision gate:** Do all three clients work in isolation against a
running localnet + Codex + Waku? If a single one is blocked (most likely Waku FFI
on macOS), evaluate: can you swap in a temporary HTTP-based bridge while still
satisfying the spec's "Logos Delivery" requirement?

---

## Phase 4 — Integration testing + CU benchmarks (Day 3-4, 4-7 hours)

Goal: prove the real backends work end-to-end and produce the CU numbers the prize
spec requires.

- [ ] **4.1** Deploy chronicle-registry to local sequencer:
  ```bash
  lgs localnet start &        # background
  lgs deploy programs/chronicle-registry/target/.../chronicle_registry.bin
  # Save the program ID
  export CHRONICLE_PROGRAM_ID=<hex>
  ```
- [ ] **4.2** Initialize the registry:
  ```bash
  lgs spel --idl docs/idl/chronicle.json --program-id $CHRONICLE_PROGRAM_ID \
    -p <bin> init_registry
  ```
- [ ] **4.3** Run the real end-to-end integration test:
  ```bash
  USE_REAL_LOGOS=1 RISC0_DEV_MODE=0 ./scripts/demo.sh
  ```
  - Acceptance: a real CID is uploaded to Codex, broadcast on Waku, anchored on LEZ,
    and queryable. Expect this to take *minutes* per anchor due to RISC0 proving time.
- [ ] **4.4** Write the CU benchmark binary at
  `programs/chronicle-registry/examples/src/bin/cu_measure.rs`:
  - Invokes `index_batch` with N=1, then N=5, 10, 25, 50
  - Reads the tx receipt from the sequencer, extracts `cycles_used`
  - Prints a markdown table
- [ ] **4.5** Run the benchmarks (slow — 30-60 min):
  ```bash
  RISC0_DEV_MODE=0 cargo run --release --example cu_measure --features real-spel > docs/CU_BENCHMARKS_RESULTS.md
  ```
- [ ] **4.6** Update `docs/CU_BENCHMARKS.md` to embed the actual numbers + a chart
  (use [Mermaid](https://mermaid.js.org/syntax/xyChart.html) line chart so it renders
  on GitHub without an image)
  - 🏆 If per-CID cost at N=50 is < 10× per-CID cost at N=1, call this out
    explicitly in the doc and the submission PR — it's the strongest possible
    answer to the spec's "performance" criterion
- [ ] **4.7** Update CI workflow to run a fast subset of integration tests in
  `RISC0_DEV_MODE=1` against the localnet (full proving is too slow for every CI run)
- [ ] **4.8** Update `README.md` "Deployment" section with:
  - Deployed program ID
  - Sequencer endpoint
  - Sample `wallet account get` command demonstrating a queryable entry

🎯 **Phase 4 decision gate:** Did `demo.sh` with `USE_REAL_LOGOS=1 RISC0_DEV_MODE=0`
succeed at least once in under 5 minutes? If proving time per call is > 5 min, the
demo recording in phase 6 becomes painful. Investigate parallel proving or accept
that the recording will need clever editing.

---

## Phase 5 — Basecamp `.lgx` package (Day 4-5, 4-8 hours)

Goal: package `app/` as an installable Basecamp module. **This is the riskiest phase
on macOS** — nix + Qt6 + the logos-module-builder toolchain has limited macOS testing.

- [ ] **5.1** Install [Nix](https://nixos.org/download) if not already (consider the
  [Determinate Systems installer](https://determinate.systems/nix-installer/) for
  better macOS support)
- [ ] **5.2** Clone `logos-co/logos-module-builder` and study its `mkLogosQmlModule`
  helper for the QML-app pattern
- [ ] **5.3** Add `flake.nix` outputs for both:
  - `packages.x86_64-darwin.doc-index-lgx-portable` (the core module)
  - `packages.x86_64-darwin.whistleblower-lgx-portable` (the QML app)
- [ ] **5.4** Build both packages:
  ```bash
  nix build .#doc-index-lgx-portable -o /tmp/doc-index.lgx
  nix build .#whistleblower-lgx-portable -o /tmp/whistleblower.lgx
  ls -lh /tmp/*.lgx
  ```
- [ ] **5.5** Clone `logos-co/logos-standalone-app` and load the modules in isolation:
  ```bash
  cd logos-standalone-app
  ./run.sh --module ~/Python\ experiments/whistleblower/app
  ```
  - Acceptance: the Basecamp window opens, the Whistleblower UI loads, the file
    picker works, and `logos.module("doc-index").publishFileJson(...)` returns a CID
- [ ] **5.6** Upload `whistleblower.lgx` as a GitHub release asset:
  ```bash
  gh release create v0.1.0 /tmp/whistleblower.lgx /tmp/doc-index.lgx \
    --title "LP-0017 submission" --notes-file docs/SUBMISSION_RELEASE_NOTES.md
  ```
- [ ] **5.7** Update README with the release-download link and the `lgpm install`
  command

🎯 **Phase 5 decision gate:** Does the `.lgx` load in standalone-app and the UI work?
If standalone-app crashes or the FFI bridge fails:
- File a GitHub issue with the exact error
- Drop the `.lgx` from the submission and ship "tested via direct module path" instead
  (the spec says "loadable in Logos app (Basecamp)" but submitters in the past have
  shipped working module dirs even when the `.lgx` archive was problematic)

---

## Phase 6 — Demo recording + Vercel polish (Day 5, 2-4 hours)

Goal: produce the narrated video walkthrough that the prize spec **explicitly requires**.

- [ ] **6.1** Re-run `scripts/demo.sh` with `USE_REAL_LOGOS=1 RISC0_DEV_MODE=0` and
  verify it completes cleanly. This is the recording target.
- [ ] **6.2** Set up the recording environment per
  [`docs/RECORDING_SCRIPT.md`](RECORDING_SCRIPT.md):
  - 1280×720 or 1440×900 browser
  - Focus mode on, notifications off
  - Terminal font ≥ 14pt so `RISC0_DEV_MODE=0` is readable
- [ ] **6.3** Record two segments:
  - **Web demo** (~60s, on Vercel URL): publish → broadcast → anchor → lookup via
    the guided tour
  - **Real CLI demo** (~2-3 min, on localhost): `RISC0_DEV_MODE=0` echoed, then
    `demo.sh` run end-to-end, showing the proof generation in terminal output, the
    program ID, the on-chain tx receipt, and the lookup result
  - ⚠️ The prize spec explicitly requires the second one: *"the recording must show
    terminal output (including proof generation) to confirm RISC0_DEV_MODE=0 was
    active"*
- [ ] **6.4** Edit into a single ~3-4 minute video. Recommended: Loom (auto-hosted) or
  upload to YouTube unlisted.
- [ ] **6.5** Update `web/app/page.tsx` Hero to add a "▶ Watch the demo" link next to
  "Run guided tour", pointing to the video URL
- [ ] **6.6** Update root `README.md` second placeholder to link the recording
- [ ] **6.7** Rebuild + redeploy to Vercel (auto-triggered by the push)

🎯 **Phase 6 decision gate:** Does the video clearly show `RISC0_DEV_MODE=0` in the
terminal AND demonstrate all four functional criteria (upload, broadcast, batch anchor,
on-chain query)? If not, re-record. **This is the single most likely cause of
submission rejection.**

---

## Phase 7 — Pre-flight + submission (Day 6, 2-4 hours)

Goal: pass the evaluator's "clean clone test" before they run it, and submit cleanly.

- [ ] **7.1** Fresh-clone test in a Docker container:
  ```bash
  docker run --rm -it -v "$PWD/clean-test:/work" rust:latest bash -c '
    cd /work
    git clone https://github.com/warfield2016/whistleblower
    cd whistleblower
    cargo test --workspace
    ./scripts/demo.sh
  '
  ```
  - Acceptance: cargo test passes; demo.sh exits 0 with no manual intervention
  - ⚠️ If the demo needs `lgs` / Codex / Waku to be running, document the prerequisite
    install steps in the README before this matters to evaluators
- [ ] **7.2** Run `cargo fmt --check` and `cargo clippy -D warnings` one final time
- [ ] **7.3** Verify CI is green on `main` on GitHub
- [ ] **7.4** File 2-3 GitHub issues against Logos repos for friction you hit during
  phases 1-5. This is a **submission requirement**: *"GitHub issues filed for any
  problems encountered with Logos technology"*. Reasonable issues:
  - SPEL framework version pin docs (if you got stuck on rev mismatch)
  - Basecamp standalone-app on macOS (if Qt issues)
  - LEZ devnet circuit download UX (if confusing)
  - Cross-link each from `docs/INTEGRATION_NOTES.md`
- [ ] **7.5** Tag a release on GitHub:
  ```bash
  git tag -a v0.1.0 -m "LP-0017 submission"
  git push origin v0.1.0
  gh release edit v0.1.0 --notes-file docs/SUBMISSION_RELEASE_NOTES.md
  ```
- [ ] **7.6** Write the submission PR description following the LP-0017 spec's
  "Submission Requirements" section verbatim — every bullet point answered with a
  link to the relevant artifact:
  - Public repo: → github.com/warfield2016/whistleblower (MIT/Apache-2.0)
  - Basecamp app: → `app/` + `gh release` link to `.lgx`
  - Document-indexing module: → `crates/doc-index-core/` + docs/API.md
  - On-chain registry: → `programs/chronicle-registry/` + program ID on devnet
  - Batch anchor CLI: → `crates/batch-anchor/`
  - Integration tests in CI: → `.github/workflows/ci.yml` + green badge
  - Deployed program address: → `<hex>` on LEZ devnet
  - Video walkthrough: → recording URL
  - CU benchmarks: → `docs/CU_BENCHMARKS.md` with table + chart
  - GitHub issues filed: → links to your filed issues from 7.4
- [ ] **7.7** Submit the PR to `logos-co/lambda-prize`. Use draft PR first to sanity
  check rendering, then mark ready for review.
- [ ] **7.8** Comment on `logos-co/ecosystem#120` (the tracking issue) noting your
  submission and PR link — increases visibility for `weboko` (the reviewer)

🎯 **Phase 7 decision gate:** Are *all* spec submission requirements answered in the
PR description with working links? Submitting with missing items wastes one of your 3
allowed submissions.

---

## After submission

- [ ] **8.1** Set a follow-up reminder for +7 days to check review status
- [ ] **8.2** Update [`project_whistleblower_lp0017`](../../../.claude/projects/-Users-warfield-Python-experiments/memory/project_whistleblower_lp0017.md)
  memory with submission outcome + reviewer feedback
- [ ] **8.3** If awarded → file follow-up λPrize ideas (search/discoverability on top
  of the indexing module, as the spec teases)
- [ ] **8.4** If rejected with actionable feedback → fix and use one of your remaining
  2 submission slots (max 1/week)
- [ ] **8.5** If superseded by competitor → keep the repo as ecosystem-entry signaling
  and pivot

---

## Aborted-sprint contingencies

If phase 1 fails or phase 0 reveals the competitor was awarded:

1. **Don't delete what you built.** The repo + WASM demo is genuine ecosystem
   contribution and has value beyond this prize.
2. **File the 2-3 GitHub issues from `docs/INTEGRATION_NOTES.md`** even without
   submitting — visible ecosystem engagement helps future LP work.
3. **Re-target.** The doc-index-core module is genuinely reusable. Pitch it as the
   foundation for a future search/discoverability λPrize (the LP-0017 spec literally
   teases this: *"A follow-up prize may extend the app with search and discoverability
   features built on top of the document-indexing module."*)

---

## Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Vercel build fails on Rust toolchain install | Low | `scripts/vercel-build.sh` uses the official rustup installer; logs visible in Vercel dashboard |
| LEZ devnet circuit download is multi-GB | High | Budget bandwidth; download in parallel with phase 2 work |
| SPEL framework rev mismatch | High | Pin to a known-working rev (the one lez-multisig uses) |
| RISC0 proof time > 5 min/anchor | Medium | Use `RISC0_DEV_MODE=1` for unit/integration tests, only switch to `0` for the recording |
| Basecamp `.lgx` build broken on macOS | High | Have a "module directory only" fallback that satisfies the spec |
| Competing PR awarded mid-sprint | Medium | Check status at every phase gate; abort cleanly if so |
| Video re-takes burn hours | Medium | Pre-script every shot; `docs/RECORDING_SCRIPT.md` does this |
| Forgot to show `RISC0_DEV_MODE=0` in terminal | Critical | This is the explicit reject criterion; check the recording before uploading |

---

## Total time budget

| Phase | Optimistic | Pessimistic |
|---|---|---|
| 0 — Pre-flight | 1h | 2h |
| 1 — Dev env | 2h | 4h |
| 2 — SPEL program | 4h | 6h |
| 3 — Real clients | 8h | 12h |
| 4 — Integration + CU | 4h | 7h |
| 5 — Basecamp `.lgx` | 4h | 8h |
| 6 — Recording + Vercel | 2h | 4h |
| 7 — Pre-flight + submit | 2h | 4h |
| **Total** | **27h** | **47h** |

At ~35h average, ~$11/hr effective rate at the $400 prize. **The economics only make
sense if you value the ecosystem positioning** — see
[`project_whistleblower_lp0017`](../../../.claude/projects/-Users-warfield-Python-experiments/memory/project_whistleblower_lp0017.md)
for the strategic framing.

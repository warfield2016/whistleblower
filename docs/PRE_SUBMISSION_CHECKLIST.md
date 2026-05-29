# Pre-submission checklist — LP-0017

Tactical punch-list of items that must flip from open to done **before** we
file the PR to `logos-co/lambda-prize`. Each item has an acceptance criterion
you can verify with a single shell command or eyeball-check.

For the strategic 7-phase plan, see [`SUBMISSION_CHECKLIST.md`](SUBMISSION_CHECKLIST.md).
For the PR body draft, see [`SUBMISSION_PR.md`](SUBMISSION_PR.md).
For the per-spec-criterion audit, see [`COMPETITOR_ANALYSIS.md`](COMPETITOR_ANALYSIS.md).

---

## 🔴 MANDATORY — without these, submission fails the spec

These map directly to explicit prize-spec criteria that produce a literal
"fail" verdict from the reviewer if missing.

### 1. Real backend clients (`crates/doc-index-core/src/clients/real.rs`)
- [ ] **1a** `CodexClient::upload` returns a real CID from `liblogosstorage`
  - Who: me
  - Time: 3-4h
  - Acceptance: `cargo test -p doc-index-core --features real-logos real_codex_smoke -- --ignored` uploads a file to a running Codex node and returns a non-empty CID string
- [ ] **1b** `WakuClient::publish` + `subscribe` round-trip an envelope via `liblogosdelivery`
  - Who: me
  - Time: 3-4h
  - Acceptance: two processes can publish/subscribe through Waku and the envelope's CID round-trips byte-identical
- [ ] **1c** `LgsAnchorClient::submit_batch` returns a real tx hash from `lgs spel ... index_batch`
  - Who: me
  - Time: 2-3h
  - Acceptance: `submit_batch(vec![single_entry])` against a running localnet returns a tx hash hex string matching `^[0-9a-f]{64}$`
- [ ] **1d** `--real-logos` flag on `batch-anchor` and `doc-index` CLIs switches to real clients
  - Who: me
  - Time: 30min
  - Acceptance: `batch-anchor --real-logos --run-for-secs 5` connects to real Waku + real LEZ sequencer; mock mode still works as default

### 2. Deploy chronicle-registry to LEZ devnet
- [ ] **2a** Deploy succeeds and captures a program ID
  - Who: me (autonomous in container)
  - Time: 1h
  - Acceptance: `lgs wallet -- deploy-program <bin>` returns a 64-hex program ID. Save it as `LOGOS_PROGRAM_ID` env var in `scripts/setup.sh`
- [ ] **2b** README and `docs/SUBMISSION_PR.md` populated with program ID and sequencer endpoint
  - Who: me
  - Time: 15min
  - Acceptance: `grep -E "(LOGOS_PROGRAM_ID|sequencer.*3040)" README.md docs/SUBMISSION_PR.md` shows actual values, no `<TODO>` placeholders
- [ ] **2c** `lgs spel inspect <pda>` returns a queryable registry account
  - Who: me
  - Time: 5min
  - Acceptance: command output shows `initialized=true` and `entries=...`

### 3. End-to-end demo with `RISC0_DEV_MODE=0`
- [ ] **3a** `scripts/demo.sh` supports `USE_REAL_LOGOS=1 RISC0_DEV_MODE=0` mode end-to-end
  - Who: me
  - Time: 1h
  - Acceptance: a single `bash scripts/demo.sh` invocation with both vars set publishes a real file, broadcasts on real Waku, anchors via real LEZ, and the lookup returns the entry. Exit code 0.
- [ ] **3b** Demo script echoes `RISC0_DEV_MODE=$RISC0_DEV_MODE` at the start so the recording captures it
  - Who: me (already done in current `scripts/demo.sh`)
  - Time: 0 (already in code)
  - Acceptance: `grep "RISC0_DEV_MODE" scripts/demo.sh` returns at least one echo line

### 4. CU benchmarks (P13 / SR23)
- [ ] **4a** Write `cu_measure.rs` runner: invokes `index_batch` with N=1, 5, 10, 25, 50; reads tx receipts; extracts cycles_used
  - Who: me
  - Time: 2-3h
  - Acceptance: `cargo run --release --bin cu_measure` produces a markdown table to stdout with 5 rows
- [ ] **4b** Run with `RISC0_DEV_MODE=0` against the deployed program; commit results
  - Who: me
  - Time: 1-2h (RISC0 proving is slow — budget ~5-10 min per measurement)
  - Acceptance: `docs/CU_BENCHMARKS_RESULTS.md` exists with a 5-row table and no `<TODO>` placeholders
- [ ] **4c** Update `docs/CU_BENCHMARKS.md` with results table + per-CID ratio analysis
  - Who: me
  - Time: 30min
  - Acceptance: `grep "TODO" docs/CU_BENCHMARKS.md` returns nothing

### 5. Recorded narrated video (S19 / SR22)
- [ ] **5a** Record web demo segment (~60s on live Vercel URL)
  - Who: **YOU** (needs human at keyboard)
  - Time: 30min including takes
  - Acceptance: video file exists locally OR Loom link works in your browser
- [ ] **5b** Record real-CLI demo segment (~2-3min) showing `RISC0_DEV_MODE=0` in terminal
  - Who: **YOU**
  - Time: 1-2h including takes
  - Acceptance: video clearly shows the env var echoed in terminal text, full proof-generation log visible, tx receipt visible, lookup result visible
  - ⚠️ **MOST-LIKELY REJECTION CAUSE**: if `RISC0_DEV_MODE=0` is not visible in the final video, the submission fails per spec. Re-record if it's cut off or blurry.
- [ ] **5c** Combine into single 3-4min video, upload to Loom or YouTube unlisted
  - Who: **YOU**
  - Time: 30min
  - Acceptance: URL plays the combined video in a fresh browser session
- [ ] **5d** Update README + `docs/SUBMISSION_PR.md` with the video URL
  - Who: me
  - Time: 5min
  - Acceptance: `grep "TODO.*video\|TODO.*Loom" README.md docs/SUBMISSION_PR.md` returns nothing

### 6. File 3 GitHub issues (SR24)
- [ ] **6a** File "Intel macOS not buildable" against `logos-co/scaffold`
  - Who: me
  - Time: 10min
  - Acceptance: issue URL returned by `gh issue create`
- [ ] **6b** File "ruint 1.18 breaks risc0 build" against `logos-blockchain/logos-execution-zone`
  - Who: me
  - Time: 10min
  - Acceptance: issue URL
- [ ] **6c** File "lez-framework template doesn't compile" against `logos-co/scaffold`
  - Who: me
  - Time: 10min
  - Acceptance: issue URL
- [ ] **6d** Update `docs/SUBMISSION_PR.md` SR24 section with the 3 issue URLs
  - Who: me
  - Time: 5min
  - Acceptance: `grep "github.com.*issues" docs/SUBMISSION_PR.md | wc -l` returns 3+

### 7. Real-sequencer integration test in CI (S15)
- [ ] **7a** Add `integration-real-sequencer` job to `.github/workflows/ci.yml` that starts `lgs localnet`, deploys, runs demo.sh
  - Who: me
  - Time: 1-2h
  - Acceptance: GitHub Actions workflow run is green for the new job on a PR commit
  - Note: this CI job uses `RISC0_DEV_MODE=1` for speed; the video records the `=0` proof

---

## 🟡 RECOMMENDED — these don't fail the spec but tilt the win-odds

### 8. IDL (U9 workaround)
- [ ] **8a** Hand-write `docs/idl/chronicle.json` with both instructions + account schema
  - Who: me
  - Time: 30min
  - Acceptance: file exists, parses as JSON, lists `init_registry` + `index_batch` with their args + types

### 9. Vercel deploy (gives us the web demo URL for the PR)
- [ ] **9a** Deploy to vercel.com/new via the dashboard
  - Who: **YOU**
  - Time: 30min including first-build wait
  - Acceptance: live URL loads, "Run guided tour" plays end-to-end
- [ ] **9b** Update README + `docs/SUBMISSION_PR.md` top with Vercel URL
  - Who: me
  - Time: 5min

### 10. `.lgx` package (U7 — riskiest item)
- [ ] **10a** Install Nix in container OR on host
- [ ] **10b** Build `doc-index-lgx-portable` + `whistleblower-lgx-portable` via flake
- [ ] **10c** Upload as `gh release create v0.1.0` assets
  - Total: 4-8h, may fail on macOS. **Acceptable fallback:** ship loadable module dirs in repo without the `.lgx` archive — flag this in PR description as "tested via direct module path"

### 11. Pre-flight clean-clone test (defends against S18 evaluator-clone failure)
- [ ] **11a** `docker run --rm rust:latest ...` clone + cargo test + demo.sh sequence
  - Who: me
  - Time: 1h
  - Acceptance: exit 0 from a container that has never seen this repo before, with no manual env var setup needed

---

## ✅ Already done (do not re-do)

- ✅ SR20 — public repo, MIT + Apache-2.0
- ✅ F3 — UI "Anchor on-chain" action in `app/qml/Main.qml`
- ✅ F6 — reusable `doc-index-core` module with `metadata.json` + `docs/API.md`
- ✅ U8 — module README + API doc
- ✅ R10 — upload retries with exponential backoff (tested)
- ✅ R11 — broadcast dedup (tested)
- ✅ R12 — batch tool resumes after interruption (tested)
- ✅ S16 — CI green (46 tests passing on `main`)
- ✅ Phase 1 — Docker dev environment, sequencer runs in container
- ✅ Phase 2 — `chronicle_registry.bin` compiles (454KB, ImageID `fa5d7382...`)

---

## The "Are we ready?" go/no-go gate

Run this exactly as written. Submit only when ALL three lines return `0`.

```bash
# Gate 1: no remaining TODOs in the PR draft
grep -cE "TODO|<🟡 \| ✅>|❌" docs/SUBMISSION_PR.md
# expected: 0

# Gate 2: every mandatory checkbox above is ticked
grep -cE "^- \[ \] \*\*[1-7][a-d]\*\*" docs/PRE_SUBMISSION_CHECKLIST.md
# expected: 0

# Gate 3: CI green on latest main
gh run list --branch main --limit 1 --json conclusion --jq '.[].conclusion'
# expected: "success"
```

When all three pass — and not before — proceed to submission steps below.

---

## Submission steps (only run when gate above passes)

- [ ] **S1** Final `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] **S2** Tag the release:
  ```bash
  git tag -a v0.1.0 -m "LP-0017 submission"
  git push origin v0.1.0
  ```
- [ ] **S3** Re-check competitor PR status one last time:
  ```bash
  gh pr view 48 --repo logos-co/lambda-prize --json state,updatedAt
  ```
  If `state: MERGED` → STOP, pivot to follow-up λPrize per `SUBMISSION_CHECKLIST.md` Aborted-sprint section.
- [ ] **S4** File the PR:
  ```bash
  gh pr create \
    --repo logos-co/lambda-prize \
    --title "Solution: LP-0017 — Whistleblower" \
    --body "$(cat docs/SUBMISSION_PR.md)" \
    --draft
  ```
  Open the draft PR in browser, eyeball it once, then mark ready-for-review.
- [ ] **S5** Comment on tracking issue:
  ```bash
  gh issue comment 120 --repo logos-co/ecosystem \
    --body "Submission filed: <PR_URL>"
  ```
- [ ] **S6** Update `MEMORY.md` + `project_whistleblower_lp0017.md` with submission status + PR URL.

---

## Time estimate to "gate passes"

| Phase | Mine (autonomous) | Yours (manual) |
|---|---|---|
| §1 real backend clients | 9-12h | — |
| §2 deploy to devnet | 1h | — |
| §3 demo script real-mode | 1h | — |
| §4 CU benchmarks | 4-6h (slow proofs) | — |
| §5 video recording | 5min (README update) | 2-3h |
| §6 file 3 issues | 30min | — |
| §7 CI integration job | 1-2h | — |
| §8 IDL JSON | 30min | — |
| §9 Vercel deploy | 5min (README update) | 30min |
| §10 .lgx (risky) | 4-8h | — |
| §11 fresh-clone test | 1h | — |
| **Total** | **22-32h** | **~3h** |

That's **~3 calendar days of focused work** if I run continuously and you do
your part (Vercel + video) in parallel.

---

## Last sanity check before submission

Ask yourself, one final time:

1. Is `RISC0_DEV_MODE=0` clearly visible in the video terminal? *(Most-rejected criterion.)*
2. Does `git clone && cd whistleblower && bash scripts/demo.sh` work in a fresh Docker container with no manual setup?
3. Are all 3 GitHub issues actually filed (not just drafted)?
4. Does the PR description's at-a-glance grid have **zero** 🟡 or ❌ markers?

If yes to all four: ship it. If no to any: don't.

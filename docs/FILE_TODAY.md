# File today — the few-hours path

**Goal:** file an honest, well-framed submission in the next few hours and move on.

**What this is:** a *partial* submission. It does NOT meet all 24 spec criteria
(real Codex/Waku FFI, public devnet deploy, CU benchmarks, and video are not
done). Filing now:
- ✅ Locks our first-come-first-served timestamp
- ✅ Gets pass/fail-per-criterion feedback from the reviewer
- ✅ Lets you close this out and move on
- ⚠️ Burns 1 of 3 lifetime submission slots
- ⚠️ Will very likely come back "fail, incomplete" on first pass

If you'd rather NOT burn a slot, see "Escape hatch" at the bottom — you can
leave the repo as a portfolio artifact without filing at all.

---

## YOU — ~40 min, only you can do these

- [ ] **Y1. Deploy the web demo to Vercel** (~30 min, optional but recommended)
  - Go to https://vercel.com/new → import `warfield2016/whistleblower`
  - Framework auto-detects Next.js; root dir `.`; click Deploy
  - Wait ~5-7 min for first build, confirm "Run guided tour" works on the live URL
  - Paste the URL back to me (or into `README.md` top + `docs/SUBMISSION_PR.md` header)
  - *Why:* a live, clickable demo is the single strongest thing a partial submission
    can show. The competitor has no web demo at all.

- [ ] **Y2. Decide who clicks "submit"**
  - Either: paste me the go-ahead and I run `gh pr create`
  - Or: you run the one command I'll hand you (in §M5 below)

- [ ] **Y3. (Optional) 15-sec screen GIF of the guided tour** (~10 min)
  - Only if you want the README to have a visual. Skip if short on time.

---

## ME — ~2 hours, autonomous, starting on your go-ahead

- [ ] **M1. File the 3 GitHub issues** (~30 min) → closes criterion SR24
  - Intel-Mac build failure → `logos-co/scaffold`
  - ruint 1.18 vs risc0 rustc → `logos-blockchain/logos-execution-zone`
  - lez-framework template broken → `logos-co/scaffold`
  - (drafts already written in `docs/INTEGRATION_NOTES.md`)

- [ ] **M2. Hand-write `docs/idl/chronicle.json`** (~30 min) → addresses criterion U9
  - The two instructions (`init_registry`, `index_batch`) + account schema in JSON

- [ ] **M3. Reframe `docs/SUBMISSION_PR.md` as an honest partial** (~45 min)
  - Replace the `<TODO>`/`<🟡 | ✅>` placeholders with a clear
    "Complete / Deferred" split
  - Lead with what's done + tested (orchestration, compiled program, module,
    web demo) and an explicit "Known incomplete, in progress" section
  - Drop the placeholders we can't fill today (devnet address, CU numbers, video)
    and label them honestly as "deferred to a follow-up commit"

- [ ] **M4. Final hygiene pass** (~15 min)
  - `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace`
  - Confirm CI green on `main`
  - `git tag -a v0.1.0-partial -m "LP-0017 partial submission"`

- [ ] **M5. Hand you the exact file-it command** (when M1-M4 done + your Vercel URL in)
  ```bash
  gh pr create \
    --repo logos-co/lambda-prize \
    --title "Solution: LP-0017 — Whistleblower (partial, seeking feedback)" \
    --body "$(cat docs/SUBMISSION_PR.md)"
  ```
  Plus a comment on tracking issue `logos-co/ecosystem#120`.

---

## The actual finish line

When these are all true, file it:

- [ ] Vercel URL is in the PR body (or you've decided to skip the demo link)
- [ ] 3 GitHub issues filed, URLs in the PR body
- [ ] `docs/SUBMISSION_PR.md` reads as an honest partial (no dangling `<TODO>`)
- [ ] `cargo test --workspace` green, CI green on `main`
- [ ] You've re-checked the competitor PR isn't already merged:
      `gh pr view 48 --repo logos-co/lambda-prize --json state`

Then: run the §M5 command → comment on the tracking issue → done. Move on.

---

## Escape hatch — don't file, just shelve (0 min, 0 slots burned)

If after reading this you'd rather not spend a slot on a partial:

- The repo stays public at `github.com/warfield2016/whistleblower` as a portfolio
  artifact — scaffold, compiled LEZ program, web demo, full planning docs.
- Nothing is lost; you can come back and finish the real-backend wiring later,
  or pivot the reusable `doc-index-core` module to a different λPrize.
- Just say "shelve it" and I'll update the memory files to reflect the decision
  and stop here.

This is a legitimate outcome given the $400-for-~25h economics. No slot burned,
full optionality retained.

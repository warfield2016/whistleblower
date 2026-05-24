# Deployment guide

## GitHub

Already done. The repo lives at **https://github.com/warfield2016/whistleblower** with `main` as the default branch.

To re-clone:

```bash
git clone https://github.com/warfield2016/whistleblower
cd whistleblower
cargo test --workspace          # 44 tests
./scripts/demo.sh               # end-to-end mock demo
```

## Vercel (web demo)

The repo is pre-configured for Vercel. The deploy is **manual one-time setup** then automatic.

### One-time setup

1. **Sign in to Vercel:** [vercel.com/new](https://vercel.com/new)
2. **Import the repo:** click "Import Git Repository", paste `https://github.com/warfield2016/whistleblower`, click Import.
3. **Configure the project** (most settings come from [`vercel.json`](../vercel.json) automatically):
   - **Framework Preset:** Next.js (auto-detected)
   - **Root Directory:** `.` (repo root — Vercel needs to access `web-demo/` and `web/`)
   - **Build Command:** leave blank (auto-uses `bash ./scripts/vercel-build.sh` from vercel.json)
   - **Output Directory:** leave blank (auto-uses `web/.next` from vercel.json)
   - **Install Command:** leave blank (auto-uses `cd web && npm install` from vercel.json)
4. **Environment variables:** none required for the mock demo.
5. **Deploy.** First build takes ~5-7 minutes (Rust toolchain install + wasm-pack install + cargo build + next build). Subsequent builds are cached and take ~2-3 minutes.

### After deploy

Vercel will give you a URL like `https://whistleblower-warfield2016.vercel.app`. Add it to:

- The repo's GitHub "About" / Website field (gear icon on the repo page)
- The README's top section so evaluators see the live demo link first
- The LP-0017 submission PR description

### Custom domain (optional)

If you want a memorable URL like `whistleblower.warfield.dev`:

1. Vercel project → Settings → Domains → Add
2. Add your DNS records as instructed
3. Vercel provisions TLS automatically

### CI: re-deploys on every push

Vercel auto-deploys every push to `main`. Pull requests get preview deployments at unique URLs — useful for reviewing UI changes before merge.

The [`vercel.json`](../vercel.json) `ignoreCommand` skips builds when only Rust crates (other than `registry-core`), tests, or docs change — those don't affect the web demo. Saves Vercel build minutes.

## Future: real Logos integration

When the real-Logos backends are wired up (see [crates/doc-index-core/src/clients/real.rs](../crates/doc-index-core/src/clients/real.rs) and the `real-spel` feature block in [programs/chronicle-registry/src/lib.rs](../programs/chronicle-registry/src/lib.rs)):

1. Local LEZ devnet via `lgs localnet start` (in a separate terminal, not Vercel).
2. Deploy chronicle-registry: `lgs deploy` — write the program ID into a `LOGOS_PROGRAM_ID` env var.
3. Set `USE_REAL_LOGOS=1` + `RISC0_DEV_MODE=0` for the demo recording.
4. Record the video demo (per LP-0017 spec, must show `RISC0_DEV_MODE=0` in terminal output).
5. Submit the PR to `logos-co/lambda-prize` with:
   - Repo URL: https://github.com/warfield2016/whistleblower
   - Live demo URL: (your Vercel URL)
   - Demo video URL
   - Deployed program ID + sequencer endpoint

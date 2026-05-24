# Contributing to Whistleblower

Thanks for your interest. This is a submission to [LP-0017](https://github.com/logos-co/lambda-prize/blob/main/prizes/LP-0017.md) — a permissionless document-indexing app for the Logos Network.

## Quick start

```bash
git clone https://github.com/warfield2016/whistleblower
cd whistleblower
cargo test --workspace           # 44 tests, ~30s on a cold cache
./scripts/demo.sh                # end-to-end mocked demo, ~15s
```

Web demo (browser-based, mocked backends):

```bash
cd web-demo && wasm-pack build --target web --out-dir ../web/lib/pkg
cd ../web && npm install && npm run dev
# → http://localhost:3000
```

## Repository layout

See [README.md](README.md) for a full file map. The strategic piece — the reusable
module — lives at [`crates/doc-index-core/`](crates/doc-index-core/). The interactive
web demo at [`web/`](web/) is what evaluators see at the Vercel URL; the production
target is the Basecamp app at [`app/`](app/).

## Development workflow

- **Format:** `cargo fmt --all` before commit. CI enforces.
- **Lint:** `cargo clippy --workspace --all-targets -- -D warnings`. CI enforces.
- **Test:** `cargo test --workspace`. New code should ship with tests in the same PR.
- **Demo:** if you touch the publish/anchor/lookup pipeline, run `./scripts/demo.sh` to confirm end-to-end.

## What's in scope vs out of scope

**In scope:**
- Bug fixes to the existing crates
- Real Logos backend integration (`crates/doc-index-core/src/clients/real.rs`)
- LEZ program improvements behind `--features real-spel` in `programs/chronicle-registry/`
- Test coverage improvements
- CU benchmark numbers once real-LEZ build is complete

**Out of scope for this prize** (welcome for follow-up λPrizes):
- Full-text search over document content
- Client-side encryption / per-publisher identity
- Content moderation / blocklists
- Cross-chain anchoring

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design rationale and
[docs/ANCHOR_CHOICE.md](docs/ANCHOR_CHOICE.md) for the LEZ-vs-zone-SDK decision.

## License

Dual MIT OR Apache-2.0. By contributing, you agree to license your contribution under
both. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

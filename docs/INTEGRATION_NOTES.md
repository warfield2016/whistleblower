# Integration notes — Logos dev environment

Friction log + workarounds discovered while running phase 1 of
[`SUBMISSION_CHECKLIST.md`](SUBMISSION_CHECKLIST.md) on macOS Intel (x86_64).
Source material for the GitHub issues to file in phase 7.4.

## Environment

| Property | Value |
|---|---|
| OS | macOS Darwin 25.5.0 (RELEASE_X86_64) |
| Architecture | **x86_64 (Intel)** — not Apple Silicon |
| Rust | rustc 1.95.0 / cargo 1.95.0 |
| RISC0 toolchain | rzup 0.5.1 + cargo-risczero (pre-installed) |
| Docker | 24.0.6 |
| Disk free | 389 GB |
| `lgs` | logos-scaffold 0.1.1 (installed from `logos-co/scaffold` HEAD) |

## Toolchain installs done in phase 1

1. **`lgs` (logos-scaffold)** — `cd scaffold && cargo install --path . --locked`.
   Takes ~3 min cold. Installs both `logos-scaffold` and `lgs` aliases under
   `~/.cargo/bin/`. Works cleanly on Intel macOS.

2. **`logos-blockchain-circuits` v0.4.1** — see workaround below.

## ⚠️ Workaround: Intel Mac circuits gap

**The blocker:** the `logos-blockchain-circuits` GitHub release only ships
four platform tarballs:
- `linux-aarch64`
- `linux-x86_64`
- `macos-aarch64` (Apple Silicon)
- `windows-x86_64`

**There is no `macos-x86_64` build.** Intel Macs are not natively supported by
the Logos toolchain release artifacts. The scaffold's `circuits.rs` tries to
auto-download `<triple>`, which fails on Intel macOS with a "no asset"
download error.

**The workaround:** the circuits archive contents are *platform-agnostic JSON
verification keys* plus *platform-specific helper binaries* (`prover`,
`verifier`, `witness_generator`). Per the scaffold's
[`circuits.rs`](https://github.com/logos-co/scaffold/blob/main/src/circuits.rs)
docs:

> scaffold-managed projects only consume the verification keys at compile
> time

So:
- The **`.json` keys** in `pol/`, `poc/`, `poq/`, `zksign/` are platform-agnostic
- The **binaries** are needed only at runtime for Circom-based zk proof
  generation, which our path (public LEZ txns + RISC0 zkVM internal proofs)
  doesn't invoke

**The escape hatch the scaffold builds in** is the `LOGOS_BLOCKCHAIN_CIRCUITS`
env var. When set, the version check is skipped and the directory is used
as-is — independent of which platform's tarball was extracted.

Concrete steps (use the `linux-x86_64` tarball on Intel macOS):

```bash
curl -L -o /tmp/circuits.tar.gz \
  "https://github.com/logos-blockchain/logos-blockchain-circuits/releases/download/v0.4.1/logos-blockchain-circuits-v0.4.1-linux-x86_64.tar.gz"
tar -xzf /tmp/circuits.tar.gz -C ~/
mv ~/logos-blockchain-circuits-v0.4.1-linux-x86_64 ~/.logos-blockchain-circuits
export LOGOS_BLOCKCHAIN_CIRCUITS="$HOME/.logos-blockchain-circuits"
# Persist:
echo 'export LOGOS_BLOCKCHAIN_CIRCUITS="$HOME/.logos-blockchain-circuits"' >> ~/.zshrc
```

Acceptance: `lgs doctor` reports `PASS | logos-blockchain-circuits`.

**The GitHub issue to file** (phase 7.4): request macos-x86_64 artifacts OR
document the linux-x86_64 cross-platform workaround in the scaffold README so
Intel Mac developers don't lose 30 minutes debugging the download failure.

## Other doctor warnings

- ⚠️ **`nix` not installed** — only required for phase 5 (`.lgx` package build
  via `logos-module-builder`). Not blocking for phases 1-4 or 6-7.
- ⚠️ Sequencer port 3040 not reachable — expected, before `lgs localnet start`.

## Scaffold-generated probe project

`lgs new probe --template lez-framework` produces this layout:

```
probe/
├── Cargo.toml                 # workspace, edition 2024, resolver 3
├── scaffold.toml              # repos.lez/spel/basecamp/lgpm pins, framework.kind = "lez-framework"
├── rust-toolchain.toml
├── .env.local
├── README.md
├── AGENTS.md                  # AI agent instructions (claude/cursor compatibility)
├── .cursor/rules/*.mdc        # Cursor IDE rules (4 files, lez-framework-template, lez-template, basecamp, lgs-cli)
├── .claude/skills/            # Claude Code skills
├── methods/
│   ├── Cargo.toml             # intermediate workspace member
│   ├── build.rs               # risc0_build::embed_methods()
│   ├── src/lib.rs             # exports the *_ELF / *_ID constants
│   └── guest/
│       ├── Cargo.toml         # guest workspace, depends on lez-framework
│       └── src/bin/lez_counter.rs   # the actual program with #[lez_program]
├── crates/lez-client-gen/     # auto-generated Rust client from IDL
├── idl/lez_counter.json       # compile-time generated IDL
├── src/
│   ├── lib.rs
│   └── bin/run_lez_counter.rs # runner / example
└── .scaffold/                 # scaffold-managed state
```

## Pinned dependency versions (from scaffold v0.2.0)

| Repo | Pin | Notes |
|---|---|---|
| `logos-blockchain/logos-execution-zone` | `35d8df0d031315219f94d1546ceb862b0e5b208f` | the LEZ source; cached at `~/Library/Caches/logos-scaffold/repos/lez/` |
| `logos-co/spel` | `ed3bbedb4b684645da05455d30a4a0be7cc4dfe0` | the SPEL framework (older parallel to lez-framework) |
| `logos-co/logos-basecamp` | `a746cdbc521f72ee22c5a4856fd17a9802bb9d69` | for phase 5 .lgx |
| `logos-co/logos-package-manager` | `e5c25989861f4487c3dc8c7b3bc0062bcbc3221f` | the `lgpm` CLI for phase 5 |
| `jimmy-claw/lez-framework` | `1e146970d2bba861a32fd3a8b4e13b1e6ff4114d` | the canonical framework (NOT SPEL — see below) |

## Framework choice: lez-framework, not SPEL

The scaffold's default template (`lez-framework`) uses
[`jimmy-claw/lez-framework`](https://github.com/jimmy-claw/lez-framework),
**not** [`logos-co/spel`](https://github.com/logos-co/spel). This corrects an
earlier assumption in `docs/COMPETITOR_ANALYSIS.md` (the competitor wrote
`spel-framework`-using code in May, but the scaffold has since moved on).

- **Macro:** `#[lez_program]` and `#[instruction]` (same names as SPEL but
  different framework)
- **Types:** `LezResult`, `LezOutput`, `LezError` (not `SpelResult` etc.)
- **Account attrs:** `#[account(init, pda = literal("name"))]`,
  `#[account(signer)]`, `#[account(mut, pda = ...)]` — identical syntax to SPEL
- **IDL:** generated at compile time via `PROGRAM_IDL_JSON` symbol, extracted
  via `lgs build idl` — no separate generator binary needed (unlike SPEL)

For our chronicle-registry implementation, use the canonical sample at:
`~/logos-toolchain/scaffold/templates/lez-framework/methods/guest/src/bin/lez_counter.rs`

## Edition + resolver requirements

The lez-framework guest crate requires **edition = "2024"** and the workspace
must use **resolver = "3"**. Our existing workspace at
`whistleblower/Cargo.toml` uses edition 2021 / resolver 2. **This is fine** —
Rust supports per-crate edition. Only the `methods/` and `methods/guest/`
crates need to match the framework's requirements; our `crates/registry-core`
etc. can stay edition 2021.

## Next probe action

Once `lgs setup` completes (cold compile of LEZ sequencer + wallet + spel
binaries from `~/Library/Caches/logos-scaffold/repos/lez/<pin>/`, takes
10-30 min), run:

```bash
lgs build       # compiles the lez_counter guest via docker
lgs localnet start
lgs deploy
lgs run --post-deploy
```

That sequence is the **feasibility gate** for phase 2 of our checklist. If it
succeeds end-to-end, we know the chronicle-registry pattern will work; if it
fails, we update this doc with the failure mode and re-plan.

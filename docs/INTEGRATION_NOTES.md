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

## ⚠️ Hard blocker: Intel Mac + Logos toolchain incompatibility

**The blocker:** the `logos-blockchain-circuits` GitHub release ships
four platform tarballs:
- `linux-aarch64`
- `linux-x86_64`
- `macos-aarch64` (Apple Silicon)
- `windows-x86_64`

**There is no `macos-x86_64` build.** Intel Macs are not natively supported by
the Logos toolchain release artifacts.

### What I tried first (the partial workaround)

I initially thought the JSON verification keys were platform-agnostic and the
binaries (`prover`, `verifier`, `witness_generator`) were only needed for
*proof generation* on the user side — which our path wouldn't invoke. Per
the scaffold's [`circuits.rs`](https://github.com/logos-co/scaffold/blob/main/src/circuits.rs) module docs:

> scaffold-managed projects only consume the verification keys at compile time

I extracted the `linux-x86_64` tarball into `~/.logos-blockchain-circuits/`
and exported `LOGOS_BLOCKCHAIN_CIRCUITS=$HOME/.logos-blockchain-circuits`.
This cleared the `lgs doctor` circuits check and let `lgs setup` succeed —
compiling the LEZ sequencer + wallet from source (2m42s).

### Why it doesn't fully work

Starting the localnet fails at runtime in the sequencer's *signing* path:

```
thread 'main' panicked at logos-blockchain.../kms/keys/zk/private.rs:57:56:
Signature should succeed:
  witness-generator command failed:
  ~/.logos-blockchain-circuits/zksign/witness_generator: cannot execute binary file
```

The sequencer's `zksign` keys subsystem invokes the `witness_generator`
binary at startup — even for public-only transactions. The
linux-x86_64 binary obviously can't execute on macOS. So the
"verification-keys-only" assumption in `circuits.rs` is incomplete: at
*runtime*, the binaries inside the circuits archive are also needed.

### Three viable paths forward

| Path | Local-dev experience | Cost | Risk |
|---|---|---|---|
| **A. Docker (everything in a Linux container)** | Edit natively in macOS, run `lgs *` inside a container. Project mounted as a volume. | Free, ~15 min setup | Docker-in-Docker for cargo-risczero may be tricky. |
| **B. GitHub Codespaces** | VS Code remote, native Linux x86_64. Free 60h/mo. | Free | Outbound network restrictions may bite when fetching Logos circuits. |
| **C. Cloud Linux VM (Hetzner / DigitalOcean)** | SSH + tmux. Asciinema for the recording. | $5-20/mo | Most engineering setup; the recording quality vs Loom on Mac is a concern. |

Path A is the lowest-friction first try since Docker is already installed.
Path B is the cleanest long-term — also has prior LP-prize precedent (see
`project_lp0013_declined` memory: "Codespaces browser demo... +4hr to
sprint, no competitor offers it"). Path C is the fallback.

### What this blocks

Phases 1.5, 2.6, 4, 6 (real-CLI demo segment), 7.1 all need a Linux env
where the witness_generator binary can actually run. The web demo
(Vercel), the mocked-pipeline tests (current 46 green), and all
Rust-only code work fine on the Mac.

### Concrete GitHub issue to file (phase 7.4)

Title: `Intel macOS not buildable — sequencer panics on witness_generator exec`

Repo: `logos-co/scaffold` (with cross-link to `logos-blockchain/logos-execution-zone`)

Body:
- The circuits release has no `macos-x86_64` asset.
- Falling back to `linux-x86_64` via `LOGOS_BLOCKCHAIN_CIRCUITS` passes
  `lgs doctor` and lets `lgs setup` complete, but the sequencer panics
  on first run in `kms/keys/zk/private.rs:57` when invoking the
  linux-built `witness_generator` binary.
- Suggested fixes: (a) ship `macos-x86_64` in `logos-blockchain-circuits`
  releases; or (b) document the requirement in scaffold README so Intel
  Mac users pivot to Docker/Codespaces early; or (c) make the scaffold's
  `doctor` check that the `witness_generator` binary is executable on
  the current platform.

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

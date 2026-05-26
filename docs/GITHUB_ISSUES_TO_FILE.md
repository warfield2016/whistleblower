# GitHub issues to file (phase 7.4)

LP-0017 submission requirement: *"GitHub issues filed for any problems
encountered with Logos technology."* Drafts here; file them against the
appropriate `logos-co/*` or `logos-blockchain/*` repo right before the
submission PR so they're fresh in the reviewer's notification feed.

---

## Issue 1 — Intel macOS not supported by toolchain

**Repo:** `logos-co/scaffold`

**Title:** Intel macOS not buildable — sequencer panics on `witness_generator` exec

**Labels:** `bug`, `platform-support`, `documentation`

**Body:**

### Environment

- macOS Darwin 25.5.0 on Intel x86_64 (not Apple Silicon, not Rosetta —
  `sysctl sysctl.proc_translated` returns unset)
- Rust 1.95.0
- rzup 0.5.1, cargo-risczero installed
- Docker 24.0.6 (for risc0 guest builds)

### Symptom

`lgs setup` succeeds (LEZ sequencer + wallet compile from source in ~3
minutes), but `lgs localnet start` panics:

```
thread 'main' panicked at logos-blockchain.../kms/keys/zk/private.rs:57:56:
Signature should succeed: Io(Custom { kind: Other, error:
  "witness-generator command failed:
   ~/.logos-blockchain-circuits/zksign/witness_generator:
   cannot execute binary file\n" })
```

### Root cause

The `logos-blockchain-circuits` release ships four platform tarballs:

- linux-aarch64
- linux-x86_64
- macos-aarch64 (Apple Silicon)
- windows-x86_64

**There is no `macos-x86_64` build.** I tried using the `linux-x86_64`
tarball with `LOGOS_BLOCKCHAIN_CIRCUITS` set to its extracted path —
this clears `lgs doctor`'s circuits check (and the scaffold's
[`circuits.rs`](https://github.com/logos-co/scaffold/blob/main/src/circuits.rs)
module docs explicitly call out the env-var branch as the escape
hatch). `lgs setup` proceeds, the LEZ source compiles, and the
sequencer + wallet binaries land in the cache.

But the sequencer's signing path at
`kms/keys/zk/private.rs:57` invokes the
`witness_generator` binary at startup, even for public-only test traffic.
The linux-built binary obviously can't exec on macOS, so the sequencer
panics on its first signing call.

So the documentation invariant "scaffold-managed projects only consume
the verification keys at compile time" is incomplete: at runtime, the
**binaries** inside the circuits archive (specifically
`zksign/witness_generator`) are also required, and they're
platform-specific.

### Suggested fixes (in preference order)

1. **Ship `macos-x86_64` artifacts in `logos-blockchain-circuits`
   releases.** Lowest user-side friction.
2. **Document the requirement explicitly in the scaffold README.** Right
   now Intel Mac users sink time discovering this (estimate from my
   session: ~30 min). A one-line "Apple Silicon only on macOS, use
   Docker / Codespaces for Intel" would save that time.
3. **Make `lgs doctor` check that
   `$LOGOS_BLOCKCHAIN_CIRCUITS/zksign/witness_generator` is executable
   on the current platform.** A `file` invocation would catch the
   Linux-binary-on-Mac case at the doctor stage rather than at sequencer
   startup.

### Workaround (what I did)

Pivoted the whole development flow to a Docker dev container running
linux-x86_64. Sample `Dockerfile` + `compose.yml` at
[warfield2016/whistleblower/docker/](https://github.com/warfield2016/whistleblower/tree/main/docker)
in case it helps anyone in the same spot.

---

## Issue 2 — Scaffold default `spel` pin doesn't exist in upstream

**Repo:** `logos-co/scaffold`

**Title:** `lgs setup` fails on spel checkout — pinned commit not in `logos-co/spel`

**Labels:** `bug`

**Body:**

### Symptom

A fresh `lgs new probe --template lez-framework` + `lgs setup` fails on
the spel clone step:

```
$ git clone --no-hardlinks -- https://github.com/logos-co/spel.git \
    ~/Library/Caches/logos-scaffold/repos/spel/ed3bbedb4b684645da05455d30a4a0be7cc4dfe0
$ git fetch --all --tags
$ git rev-parse --verify ed3bbedb4b684645da05455d30a4a0be7cc4dfe0^{commit}
error: configured spel pin ed3bbedb4b684645da05455d30a4a0be7cc4dfe0 is
not available in /Users/.../logos-scaffold/repos/spel/... from source
`https://github.com/logos-co/spel.git`. Ensure the repo source contains
this commit (try `--lez-path` pointing to a repo that has it).
```

### Context

- `scaffold.toml` lists `[repos.spel].pin = "ed3bbedb4b684645da05455d30a4a0be7cc4dfe0"`
- `git ls-remote https://github.com/logos-co/spel` does not show that SHA
  in any branch/tag
- `lgs setup` continues anyway (exit code 0) but `lgs doctor` reports
  `FAIL | repo spel | pin=ed3bbedb..., head=d24dbaac...` (the HEAD
  doesn't match the pin)

### Impact

Users following the canonical scaffold flow (`lgs new --template
lez-framework`) get a confusing FAIL in `lgs doctor` on first run, even
though the lez-framework template doesn't *actually* depend on the spel
binary for compile/deploy/localnet flows. The error message points to
`--lez-path` which is misleading (the issue is the spel pin, not the
lez path).

### Suggested fix

Bump the `[repos.spel].pin` default to an SHA that exists on
`logos-co/spel:main`, or — if the lez-framework template truly doesn't
need spel — make the spel section of the scaffold's setup/doctor
conditional on the framework kind. Right now the `[framework].kind =
"lez-framework"` declaration in `scaffold.toml` doesn't disable spel
work even though it's not required.

### Workaround

`lgs build/deploy/localnet` for lez-framework projects works fine
without spel, just with an annoying FAIL in `doctor`.

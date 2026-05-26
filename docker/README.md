# Docker dev container

A Linux x86_64 container that hosts the full Logos LEZ toolchain
(rust + risc0 + lgs + circuits) for builds that can't run natively on
Intel macOS. See [`../docs/INTEGRATION_NOTES.md`](../docs/INTEGRATION_NOTES.md)
for the blocker that made this necessary.

## One-time build

```bash
# From repo root:
docker compose -f docker/compose.yml build
```

First build: ~10-20 minutes (Rust + risc0 + scaffold + circuits).
Subsequent builds use Docker layer cache and complete in seconds.

## Interactive shell

```bash
# From repo root:
docker compose -f docker/compose.yml run --rm dev
```

Drops you into a Linux x86_64 bash shell with `lgs`, `cargo`,
`cargo-risczero`, `rzup`, `wasm-pack`, `docker` (host daemon),
`sqlite3`, etc. The whistleblower repo is mounted at `/work` — edits
made here are persisted to your host filesystem.

Inside the container:

```bash
cd /work
lgs --version     # logos-scaffold 0.x.x
cargo --version
echo $LOGOS_BLOCKCHAIN_CIRCUITS    # /root/.logos-blockchain-circuits
```

## The DooD pattern

This container mounts the host's `/var/run/docker.sock` so `cargo risczero
build` (which spawns RISC0 guest-build containers internally) talks to the
*host* Docker daemon rather than trying to run a nested daemon. This is
"Docker-on-Docker" — the spawned containers are siblings of this dev
container, not nested children.

**Implication:** the RISC0 guest build outputs land in
`/work/methods/guest/target/...` which is the host's
`./methods/guest/target/...` — so build artifacts are shared.

## Long-lived caches

Three named volumes survive container restarts so the LEZ source compile
(~2m42s) only runs once:

| Volume | Contents |
|---|---|
| `cargo-registry` | crates.io index + downloaded source crates |
| `cargo-git` | git-dependency clones (LEZ source) |
| `scaffold-cache` | `lgs` scaffold's per-project cache root (sequencer binary, wallet binary, IDL outputs) |

To nuke them and start fresh:

```bash
docker compose -f docker/compose.yml down -v
```

## Inside the container: the phase 1.5 sequence

Once you're in the shell:

```bash
# Smoke-test scaffold flow with a probe project (~5 min cold)
cd /tmp
lgs new probe --template lez-framework
cd probe
lgs setup           # ~2-3 min (LEZ source cached on host volume after first run)
lgs doctor          # should be all PASS now that we're on linux-x86_64
lgs localnet start  # starts the sequencer on :3040 — port mapped to host
# (in another container shell) test deploy
lgs build           # compiles lez_counter via docker risczero
lgs deploy
```

If that sequence succeeds end-to-end, **the feasibility gate is cleared**
and we can proceed with phase 2 (port chronicle-registry into the
lez-framework pattern).

## Running the sequencer + accessing from the host

The sequencer listens on `127.0.0.1:3040` inside the container; we map
that port to `localhost:3040` on the host. So:

- The web demo at `localhost:3000` (Next.js dev) talks to the sequencer
  at `localhost:3040` — same as it would natively.
- The batch-anchor binary running on the host can talk to the same
  sequencer — useful for the recorded demo where part of the flow runs
  natively (file browser, screen capture) while the LEZ portion runs in
  the container.

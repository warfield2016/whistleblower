# Dev container for the Logos LEZ toolchain.
#
# Intel Macs aren't natively supported by the Logos toolchain (no
# macos-x86_64 circuits release; the Linux witness_generator binary
# can't exec on macOS). This image runs everything on linux-x86_64
# while letting you edit code natively on the host.
#
# Docker-on-Docker pattern: we mount the host /var/run/docker.sock
# into this container so cargo risczero build (which spawns its own
# containers) talks to the HOST daemon, not a nested one.
#
# Usage (from repo root):
#   docker compose -f docker/compose.yml build       # ~10-20 min cold
#   docker compose -f docker/compose.yml run --rm dev   # interactive shell

FROM rust:1.95-bookworm

ENV DEBIAN_FRONTEND=noninteractive

# -----------------------------------------------------------------------------
# OS dependencies
# -----------------------------------------------------------------------------
# build-essential + clang: required by logos-blockchain crates.
# libssl-dev + pkg-config: required by reqwest / tokio-rustls.
# sqlite + libsqlite3-dev: for batch-anchor's idempotency state.
# lsof + procps + kill: used by `lgs doctor` and `lgs localnet` lifecycle.
# docker.io client only (no daemon): host socket provides the daemon.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        libclang-dev \
        libssl-dev \
        pkg-config \
        curl \
        ca-certificates \
        git \
        lsof \
        procps \
        sqlite3 \
        libsqlite3-dev \
        sudo \
        docker.io \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# risc0 toolchain (rzup + cargo-risczero)
# -----------------------------------------------------------------------------
# The `curl | bash` installer drops binaries under ~/.risc0/bin. We then have
# rzup install the platform-matched rust toolchain. Linux x86_64 here — no
# witness_generator-style cross-platform issue.
RUN curl -L https://risczero.com/install | bash
ENV PATH="/root/.risc0/bin:${PATH}"
RUN rzup install rust && rzup install cargo-risczero

# -----------------------------------------------------------------------------
# logos-scaffold (lgs) CLI
# -----------------------------------------------------------------------------
# Pinned at HEAD here for reproducibility — bump explicitly when upgrading.
# Installed into /usr/local/cargo/bin which is on PATH for the rust:1.95 image.
RUN git clone --depth 1 https://github.com/logos-co/scaffold /opt/scaffold \
    && cd /opt/scaffold \
    && cargo install --path . --locked --root /usr/local \
    && rm -rf /opt/scaffold/target

# -----------------------------------------------------------------------------
# logos-blockchain-circuits
# -----------------------------------------------------------------------------
# We bake the linux-x86_64 release into the image so first-run isn't blocked
# on network. The release URL is pinned via build arg so this image can be
# rebuilt for a different circuits version without editing the Dockerfile.
ARG CIRCUITS_VERSION=v0.4.1
ARG CIRCUITS_URL=https://github.com/logos-blockchain/logos-blockchain-circuits/releases/download/${CIRCUITS_VERSION}/logos-blockchain-circuits-${CIRCUITS_VERSION}-linux-x86_64.tar.gz
RUN mkdir -p /root/.logos-blockchain-circuits \
    && curl -fsSL "${CIRCUITS_URL}" -o /tmp/circuits.tar.gz \
    && tar -xzf /tmp/circuits.tar.gz --strip-components=1 -C /root/.logos-blockchain-circuits \
    && rm /tmp/circuits.tar.gz \
    && ls /root/.logos-blockchain-circuits/pol/verification_key.json   # sanity check
ENV LOGOS_BLOCKCHAIN_CIRCUITS=/root/.logos-blockchain-circuits

# -----------------------------------------------------------------------------
# wasm target (for the web-demo crate)
# -----------------------------------------------------------------------------
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-pack --locked

# -----------------------------------------------------------------------------
# Working area
# -----------------------------------------------------------------------------
# The whistleblower repo gets mounted at /work (see compose.yml).
# scaffold caches live in /root/.cache/logos-scaffold (volume-mounted).
WORKDIR /work

# Helpful default — show env state on shell start.
RUN echo 'echo "=== Logos dev container ==="\n\
echo "lgs:          $(lgs --version 2>&1 | head -1)"\n\
echo "cargo:        $(cargo --version)"\n\
echo "rzup:         $(rzup --version 2>&1)"\n\
echo "circuits:     $LOGOS_BLOCKCHAIN_CIRCUITS"\n\
echo "docker host:  $(docker version --format \"{{.Server.Version}}\" 2>/dev/null || echo unavailable)"\n\
echo' > /root/.bashrc.append \
    && cat /root/.bashrc.append >> /root/.bashrc

CMD ["/bin/bash"]

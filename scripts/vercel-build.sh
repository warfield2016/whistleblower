#!/usr/bin/env bash
# Vercel build script: installs the Rust toolchain + wasm-pack, builds the WASM crate,
# then builds the Next.js app.
#
# Vercel build containers come with Node preinstalled but no Rust. We install both into
# the build env and cache nothing (Vercel handles caching at the directory level).

set -euo pipefail

echo "==> Whistleblower Vercel build"
echo "    pwd: $(pwd)"
echo "    node: $(node --version 2>/dev/null || echo missing)"

# --- Install Rust if not present ---
if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Installing Rust"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

echo "    cargo: $(cargo --version)"
rustup target add wasm32-unknown-unknown

# --- Install wasm-pack if not present ---
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "==> Installing wasm-pack"
  # Binary install is faster than `cargo install wasm-pack` in CI.
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi
echo "    wasm-pack: $(wasm-pack --version)"

# --- Build the WASM module ---
echo "==> Building web-demo (Rust → WASM)"
cd web-demo
wasm-pack build --target web --out-dir ../web/lib/pkg --release
cd ..
echo "    wasm artifacts:"
ls -la web/lib/pkg/

# --- Build the Next.js app ---
echo "==> Building Next.js app"
cd web
npm run build

echo "==> Vercel build complete"

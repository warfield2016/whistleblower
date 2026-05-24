#!/usr/bin/env bash
# Whistleblower end-to-end demo.
#
# This script is the executable form of the prize spec's success criteria.
# It runs the full pipeline against the in-process mock backends — sufficient to
# demonstrate the architecture and produce a runnable artifact today, before the
# real Logos backends are wired in.
#
# Once the `real-logos` integration is done, set USE_REAL_LOGOS=1 and ensure:
#   - a LEZ sequencer is running locally (`lgs localnet start`)
#   - the chronicle-registry program is deployed (`lgs deploy`)
#   - a Codex node and Waku node are reachable
#   - RISC0_DEV_MODE=0 (mandatory for prize submission — see PRIZE_NOTES below)
#
# PRIZE_NOTES
# ===========
# The prize spec requires a recorded video demo with RISC0_DEV_MODE=0 visible in
# terminal output. This script echoes the env var at the start so the recording
# captures it. Do NOT remove that echo — it's the evaluator's only way to confirm
# the demo wasn't running with dev mode shortcuts.

set -euo pipefail

cd "$(dirname "$0")/.."

echo "============================================================"
echo " Whistleblower demo"
echo "============================================================"
echo " RISC0_DEV_MODE = ${RISC0_DEV_MODE:-<unset>}"
echo " USE_REAL_LOGOS = ${USE_REAL_LOGOS:-<unset>}"
echo " Working dir    = $(pwd)"
echo " Date           = $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "============================================================"

if [[ "${USE_REAL_LOGOS:-0}" == "1" && "${RISC0_DEV_MODE:-1}" != "0" ]]; then
    echo "ERROR: USE_REAL_LOGOS=1 requires RISC0_DEV_MODE=0 (got '${RISC0_DEV_MODE:-unset}')." >&2
    echo "       The prize submission demo must run with production proofs." >&2
    exit 2
fi

# Working dir for ephemeral state (sqlite, test file, etc).
WORK_DIR="$(mktemp -d -t whistleblower-demo.XXXXXX)"
trap 'rm -rf "$WORK_DIR"; jobs -p | xargs -r kill 2>/dev/null || true' EXIT

echo
echo "[1/6] Building workspace…"
cargo build --release --bin doc-index --bin batch-anchor

DOC_INDEX="./target/release/doc-index"
BATCH_ANCHOR="./target/release/batch-anchor"

# A nontrivial test document.
TEST_FILE="$WORK_DIR/leaked-memo.txt"
cat > "$TEST_FILE" <<'EOF'
INTERNAL — Q3 FY26
Subject: Operational risk briefing
This file demonstrates the Whistleblower upload → broadcast → anchor pipeline.
The CID printed below should appear on chronicle-registry after the batch anchor flushes.
EOF
echo "       test file: $TEST_FILE ($(wc -c < "$TEST_FILE") bytes)"

echo
echo "[2/6] Starting batch-anchor daemon in background…"
echo "       (subscribes to the Waku topic, flushes every 2s, exits after 10s)"
RUST_LOG=info "$BATCH_ANCHOR" \
    --state-file "$WORK_DIR/batch.db" \
    --flush-interval-secs 2 \
    --flush-size 5 \
    --run-for-secs 10 \
    > "$WORK_DIR/batch.log" 2>&1 &
BATCH_PID=$!
echo "       pid: $BATCH_PID"
sleep 1

echo
echo "[3/5] Running publish → anchor → lookup pipeline (single process)…"
# Use the `demo` subcommand so all three steps share the same Indexer instance
# (separate CLI invocations would each get fresh mocks — unrepresentative of how
# real Logos backends share state across clients).
"$DOC_INDEX" demo "$TEST_FILE" --title "Q3 operational briefing"

echo
echo "[4/5] Demonstrating per-invocation mock-state isolation (expected to return null)…"
# This intentionally returns null: a fresh CLI invocation has its own mock anchor
# state that doesn't include the CID anchored above. With real Logos backends, both
# invocations would hit the same on-chain registry and the lookup would succeed.
"$DOC_INDEX" lookup "zINTENTIONALLY_NOT_ANCHORED_TO_DEMONSTRATE_FRESH_STATE_BEHAVIOR" || true

echo
echo "[5/5] Waiting for batch-anchor to finish its run window…"
wait "$BATCH_PID" || true
echo
echo "       batch-anchor log:"
sed 's/^/         | /' "$WORK_DIR/batch.log"

echo
echo "============================================================"
echo " Demo complete."
echo
echo " What this demonstrated:"
echo "   - doc-index-cli publishes a file → produces CID + envelope hash"
echo "   - same CLI anchors the CID via the doc-index-core Indexer"
echo "   - lookup confirms the on-chain registry contains the CID"
echo "   - batch-anchor daemon runs as a separate process with its own state file"
echo
echo " What this does NOT yet demonstrate (real-Logos integration is the next step):"
echo "   - actual Codex upload (currently mock storage)"
echo "   - actual Waku broadcast that the batch-anchor instance can subscribe to"
echo "   - actual LEZ sequencer transaction (currently mock anchor)"
echo "   - RISC0 proof generation (currently no zkVM call)"
echo
echo " To enable the production path, see scripts/demo.sh:USE_REAL_LOGOS"
echo " and crates/doc-index-core/src/clients/real.rs (wire-up stub)."
echo "============================================================"

# Whistleblower — top-level Make targets.
#
# Inspired by competitor patterns + extended with our additions (web demo, fmt-check).
# Quick start:
#   make build idl deploy setup
#   make web                # build + serve the WASM demo
#   make test               # cargo test --workspace
#   make ci                 # fmt + clippy + test + demo-smoke

SHELL := /bin/bash
STATE_FILE := .whistleblower-state
IDL_FILE := docs/idl/chronicle.json
PROGRAMS_DIR := methods/guest/target/riscv32im-risc0-zkvm-elf/docker
PROGRAM_BIN := $(PROGRAMS_DIR)/chronicle_registry.bin
WASM_OUT := web/lib/pkg

# Persist signer ID / program ID etc across Make invocations.
-include $(STATE_FILE)

# Borrowed verbatim from Thompsonmina/WhistleBlower-Logos- — clean state persistence.
define save_var
	@grep -v '^$(1)=' $(STATE_FILE) 2>/dev/null > $(STATE_FILE).tmp || true
	@echo '$(1)=$(2)' >> $(STATE_FILE).tmp
	@mv $(STATE_FILE).tmp $(STATE_FILE)
endef

.PHONY: help build idl cli deploy setup inspect status clean test fmt clippy ci web web-clean demo

help: ## Show this help
	@echo "Whistleblower — top-level Make targets"
	@echo ""
	@echo "  Build / deploy:"
	@echo "    make build     Build the chronicle-registry guest binary (needs risc0 + spel toolchain)"
	@echo "    make idl       Generate IDL JSON from the program source"
	@echo "    make deploy    Deploy the program to the local sequencer"
	@echo "    make setup     Bootstrap: build + deploy + mint signer + init registry"
	@echo ""
	@echo "  Run:"
	@echo "    make cli ARGS= Run the IDL-driven CLI (e.g. ARGS='lookup --cid zXYZ')"
	@echo "    make demo      Run scripts/demo.sh end-to-end against mocks"
	@echo "    make web       Build WASM + start Next.js dev server on :3040"
	@echo ""
	@echo "  Quality:"
	@echo "    make test      cargo test --workspace"
	@echo "    make fmt       cargo fmt --all -- --check"
	@echo "    make clippy    cargo clippy --workspace --all-targets -- -D warnings"
	@echo "    make ci        fmt + clippy + test + demo-smoke"
	@echo ""
	@echo "  Misc:"
	@echo "    make inspect   Show program ID for the built guest binary"
	@echo "    make status    Show saved state + binary / IDL presence"
	@echo "    make clean     Remove saved state"

build: ## Build the chronicle-registry guest binary
	cargo risczero build --manifest-path methods/guest/Cargo.toml
	@echo ""
	@echo "✅ Guest binary built: $(PROGRAM_BIN)"
	@ls -la $(PROGRAM_BIN) 2>/dev/null || true

idl: ## Generate IDL JSON from the program source
	@mkdir -p $(dir $(IDL_FILE))
	cargo run --bin generate_idl > $(IDL_FILE)
	@echo "✅ IDL written to $(IDL_FILE)"

cli: ## Run the IDL-driven CLI (ARGS="...")
	cargo run --bin doc-index -- $(ARGS)

deploy: ## Deploy program to sequencer (idempotent — same binary = same program_id)
	@test -f "$(PROGRAM_BIN)" || (echo "ERROR: Binary not found. Run 'make build' first."; exit 1)
	lgs wallet -- deploy-program $(PROGRAM_BIN)
	@echo "✅ Program deployed"

setup: ## Bootstrap: requires sequencer running on :3040
	@./scripts/setup.sh

inspect: ## Show program ID for the built binary
	cargo run --bin doc-index -- inspect $(PROGRAM_BIN)

status: ## Show saved state + binary / IDL presence
	@echo "Whistleblower Status"
	@echo "──────────────────────────────────────"
	@if [ -f "$(STATE_FILE)" ]; then cat $(STATE_FILE); else echo "(no state — run 'make setup')"; fi
	@echo ""
	@echo "Guest binary:"
	@ls -la $(PROGRAM_BIN) 2>/dev/null || echo "  $(PROGRAM_BIN): NOT BUILT (run 'make build')"
	@echo ""
	@echo "IDL:"
	@ls -la $(IDL_FILE) 2>/dev/null || echo "  $(IDL_FILE): NOT GENERATED (run 'make idl')"
	@echo ""
	@echo "WASM:"
	@ls -la $(WASM_OUT)/web_demo_bg.wasm 2>/dev/null || echo "  $(WASM_OUT)/: NOT BUILT (run 'make web')"

clean: ## Remove saved state
	rm -f $(STATE_FILE) $(STATE_FILE).tmp
	@echo "✅ State cleaned"

# ── Test / lint ───────────────────────────────────────────────────────

test: ## cargo test --workspace
	cargo test --workspace

fmt: ## cargo fmt --check
	cargo fmt --all -- --check

clippy: ## cargo clippy with -D warnings
	cargo clippy --workspace --all-targets -- -D warnings

ci: fmt clippy test ## fmt + clippy + test (matches .github/workflows/ci.yml)
	@./scripts/demo.sh > /dev/null
	@echo "✅ CI gate: fmt + clippy + test + demo-smoke all green"

# ── Web demo ──────────────────────────────────────────────────────────

web: $(WASM_OUT)/web_demo_bg.wasm ## Build WASM + start Next.js dev server on :3040
	@cd web && npx next dev --port 3040

$(WASM_OUT)/web_demo_bg.wasm: web-demo/src/lib.rs web-demo/Cargo.toml
	cd web-demo && wasm-pack build --target web --out-dir ../$(WASM_OUT) --release

web-clean: ## Remove WASM build artifacts
	rm -rf $(WASM_OUT)
	@echo "✅ WASM artifacts cleaned"

demo: ## Run scripts/demo.sh end-to-end against mocks
	@./scripts/demo.sh

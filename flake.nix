{
  description = "Whistleblower — censorship-resistant document upload and indexing for the Logos Basecamp.";

  # NOTE: this is a scaffold flake. Once the Logos build deps are available (logos-module-builder,
  # logos-package-manager, spel-framework, etc.) we extend `packages` to produce:
  #
  #   .#doc-index-lgx-portable      — the reusable headless module as a .lgx
  #   .#whistleblower-lgx-portable  — the Basecamp app as a .lgx
  #   .#chronicle-registry-elf      — the LEZ program RISC0 ELF
  #
  # See logos-co/scaffold/flake.nix for the canonical pattern.

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust = pkgs.rust-bin.stable.latest.default;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rust
            pkgs.pkg-config
            pkgs.openssl
            pkgs.sqlite
          ];

          shellHook = ''
            echo "Whistleblower dev shell"
            echo "  cargo test --workspace                  # run all tests"
            echo "  ./scripts/demo.sh                       # end-to-end mock demo"
            echo "  cargo run -p doc-index-cli -- --help    # CLI usage"
          '';
        };

        # cargo-built artifacts that don't need the Logos dev env.
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "whistleblower";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl pkgs.sqlite ];
        };
      });
}

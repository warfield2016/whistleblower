//! Re-exports for the chronicle-registry guest ELF + image ID.
//!
//! After `cargo build` runs the `risc0-build` build script in this crate,
//! it generates a `methods.rs` file under `OUT_DIR` containing
//! `CHRONICLE_REGISTRY_ELF` and `CHRONICLE_REGISTRY_ID` constants. The
//! `include!` below pulls those in and re-exports them so downstream crates
//! can write `whistleblower_methods::CHRONICLE_REGISTRY_ELF`.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

// Drives the risc0 guest build. After this runs, the guest ELF + image-id are
// available as `${CARGO_PKG_NAME}_ELF` and `${CARGO_PKG_NAME}_ID` constants in
// the host-side crate that depends on `whistleblower_methods`.
//
// In our project, the off-chain client doesn't actually consume the ELF at
// runtime — `lgs deploy` reads it directly from the docker-built artifact
// path. But we keep the build.rs in place so the workspace has the canonical
// LEZ shape: any host crate that wants the program ID at compile time
// (e.g., the integration tests) can `cargo add whistleblower_methods` and
// reference the constants.
fn main() {
    risc0_build::embed_methods();
}

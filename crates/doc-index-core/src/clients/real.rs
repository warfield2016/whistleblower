//! Real Logos backend implementations.
//!
//! Gated behind the `real-logos` feature so the workspace compiles without the Logos
//! FFI dependencies in CI. This file is the integration-phase work surface — fill in
//! the bodies of these clients once a Logos dev environment is set up locally.
//!
//! ## Expected wire-up
//!
//! - [`CodexClient`] — FFI bridge to `liblogosstorage` (auto-loaded by Basecamp). Calls
//!   `uploadFile(path, contentType)` and reads the emitted CID from the async result.
//! - [`WakuClient`] — FFI bridge to `liblogosdelivery`. Calls `send(topic, payload)`
//!   for publish, `subscribe(topic)` and a message-received signal for receive.
//! - [`AnchorClient`] — shells out to `lgs spel --idl <idl> --program-id <hex> index_batch ...`
//!   and parses the JSON receipt for the tx hash.
//!
//! See `docs/ARCHITECTURE.md` for the full data flow and `docs/API.md` for the wire schemas.

use async_trait::async_trait;
use registry_core::{EntryRequest, Envelope, RegistryEntry};

use super::{
    AnchorClient, AnchorError, Cid, DeliveryClient, DeliveryError, StorageClient, StorageError,
    TxHash,
};

/// FFI-backed Codex client. Stub — calls return `NotConfigured` until wired up.
pub struct CodexClient {
    // FFI handle, config, etc.
}

#[async_trait]
impl StorageClient for CodexClient {
    async fn upload(&self, _bytes: &[u8]) -> Result<Cid, StorageError> {
        Err(StorageError::Permanent(
            "CodexClient not yet wired — enable `real-logos` and complete src/clients/real.rs"
                .into(),
        ))
    }

    async fn download(&self, _cid: &Cid) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::Permanent("CodexClient not yet wired".into()))
    }
}

/// FFI-backed Waku client. Stub.
pub struct WakuClient {
    // FFI handle, content topic, etc.
}

#[async_trait]
impl DeliveryClient for WakuClient {
    async fn publish(&self, _topic: &str, _envelope: &Envelope) -> Result<(), DeliveryError> {
        Err(DeliveryError::Network(
            "WakuClient not yet wired — enable `real-logos` and complete src/clients/real.rs"
                .into(),
        ))
    }

    async fn subscribe(
        &self,
        _topic: &str,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<Envelope>, DeliveryError> {
        Err(DeliveryError::Network("WakuClient not yet wired".into()))
    }
}

/// Shells out to `lgs spel ... index_batch` to submit the batch. Stub.
pub struct LgsAnchorClient {
    // sequencer URL, program ID, signer account, IDL path.
}

#[async_trait]
impl AnchorClient for LgsAnchorClient {
    async fn submit_batch(&self, _entries: Vec<EntryRequest>) -> Result<TxHash, AnchorError> {
        Err(AnchorError::NotConfigured("LgsAnchorClient stub"))
    }

    async fn lookup(&self, _cid: &str) -> Result<Option<RegistryEntry>, AnchorError> {
        Err(AnchorError::NotConfigured("LgsAnchorClient stub"))
    }
}

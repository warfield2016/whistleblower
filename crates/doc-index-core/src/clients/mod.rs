//! Backend traits and implementations.
//!
//! Three traits define everything the [`Indexer`](crate::Indexer) needs from the outside world:
//!
//! - [`StorageClient`] — upload bytes, get a CID back. Backed by Codex in production.
//! - [`DeliveryClient`] — publish/subscribe on a topic. Backed by Waku in production.
//! - [`AnchorClient`] — submit a batch of (cid, metadata_hash) entries to the LEZ registry.
//!
//! The [`mock`] submodule provides in-process implementations sufficient for unit tests and
//! development without any Logos infrastructure. The [`real`] submodule contains stubs that
//! will be filled in during integration phase (when the `real-logos` feature is enabled).

use async_trait::async_trait;
use registry_core::{EntryRequest, Envelope};

pub mod mock;
#[cfg(feature = "real-logos")]
pub mod real;

/// CID returned by a [`StorageClient`] upload. Codex format: multibase-encoded string.
pub type Cid = String;
/// Hash of an on-chain transaction. Hex-encoded string for portability.
pub type TxHash = String;

/// Upload arbitrary bytes to durable content-addressed storage and receive a CID.
#[async_trait]
pub trait StorageClient: Send + Sync {
    async fn upload(&self, bytes: &[u8]) -> Result<Cid, StorageError>;
    async fn download(&self, cid: &Cid) -> Result<Vec<u8>, StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("transient storage error: {0}")]
    Transient(String),
    #[error("permanent storage error: {0}")]
    Permanent(String),
    #[error("cid not found: {0}")]
    NotFound(String),
}

impl StorageError {
    /// Used by the retry logic in [`Indexer::publish_file`](crate::Indexer::publish_file) to
    /// decide whether to back off and try again or surface the failure.
    pub fn is_transient(&self) -> bool {
        matches!(self, StorageError::Transient(_))
    }
}

/// Publish a serialized envelope to a content topic, or subscribe to receive envelopes.
///
/// `subscribe` returns a stream-like channel. Implementations should fan out one channel per
/// subscriber so multiple consumers of the same topic don't steal each other's messages.
#[async_trait]
pub trait DeliveryClient: Send + Sync {
    async fn publish(&self, topic: &str, envelope: &Envelope) -> Result<(), DeliveryError>;
    async fn subscribe(
        &self,
        topic: &str,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<Envelope>, DeliveryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    #[error("delivery network error: {0}")]
    Network(String),
    #[error("encoding error: {0}")]
    Encoding(String),
}

/// Submit a batch of registry entries to chronicle-registry on LEZ.
#[async_trait]
pub trait AnchorClient: Send + Sync {
    async fn submit_batch(&self, entries: Vec<EntryRequest>) -> Result<TxHash, AnchorError>;
    async fn lookup(&self, cid: &str) -> Result<Option<registry_core::RegistryEntry>, AnchorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AnchorError {
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("invalid batch: {0}")]
    InvalidBatch(String),
    #[error("not yet configured: missing {0}")]
    NotConfigured(&'static str),
}

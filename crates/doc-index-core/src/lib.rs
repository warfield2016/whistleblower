//! Reusable document-indexing module for Logos apps.
//!
//! Exposes a single [`Indexer`] façade that orchestrates the **upload → broadcast → anchor**
//! pipeline over pluggable backends. Backends implement the [`StorageClient`], [`DeliveryClient`],
//! and [`AnchorClient`] traits — the crate ships in-process mock implementations and stubs for
//! real Logos backends (Codex / Waku / SPEL CLI).
//!
//! ## Quick start
//!
//! ```no_run
//! use doc_index_core::{Indexer, clients::mock};
//!
//! # async fn run() -> anyhow::Result<()> {
//! let indexer = Indexer::new(mock::storage(), mock::delivery(), mock::anchor());
//! let receipt = indexer
//!     .publish_file(b"hello world", doc_index_core::PublishRequest {
//!         title: "memo".into(),
//!         description: "...".into(),
//!         content_type: "text/plain".into(),
//!         tags: vec!["test".into()],
//!         broadcast: true,
//!     })
//!     .await?;
//! println!("cid={}, publish_id={}", receipt.cid, receipt.publish_id);
//! # Ok(()) }
//! ```
//!
//! ## Architecture
//!
//! See [`crate::clients`] for the backend trait definitions, [`Indexer`] for the orchestration
//! logic, and the workspace's `docs/ARCHITECTURE.md` for the system-level design.

pub mod clients;
pub mod ffi;
pub mod indexer;

pub use clients::{AnchorClient, DeliveryClient, StorageClient};
pub use indexer::{
    AnchorReceipt, Indexer, IndexerError, PublishReceipt, PublishRequest, PublishedRecord,
};

// Re-export the wire types so consumers can build envelopes without importing registry-core directly.
pub use registry_core::{
    format_metadata_hash, metadata_hash, parse_metadata_hash, EntryRequest, Envelope, Instruction,
    RegistryEntry, DEFAULT_WAKU_TOPIC, MAX_BATCH_ENTRIES, METADATA_HASH_LEN, METADATA_HASH_PREFIX,
};

//! In-process mock implementations of the backend traits.
//!
//! Sufficient for unit tests, dev environments, and the architecture-validation phase before
//! the real Logos backends are wired in. Storage is a hashmap, delivery is a per-topic
//! tokio broadcast channel, anchor is a Vec behind a Mutex.
//!
//! These are NOT performance-optimized — they're for correctness validation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use registry_core::{EntryRequest, Envelope, RegistryEntry};
use tokio::sync::{mpsc, Mutex, RwLock};

use super::{
    AnchorClient, AnchorError, Cid, DeliveryClient, DeliveryError, StorageClient, StorageError,
    TxHash,
};

/// Convenience: construct a fresh in-memory storage mock.
pub fn storage() -> Arc<MockStorage> {
    Arc::new(MockStorage::default())
}

/// Convenience: construct a fresh in-process delivery mock.
pub fn delivery() -> Arc<MockDelivery> {
    Arc::new(MockDelivery::default())
}

/// Convenience: construct a fresh in-memory anchor mock.
pub fn anchor() -> Arc<MockAnchor> {
    Arc::new(MockAnchor::default())
}

#[derive(Default)]
pub struct MockStorage {
    inner: RwLock<HashMap<Cid, Vec<u8>>>,
    /// Set this >0 to make the next N `upload` calls fail with `Transient`. Used by tests
    /// that exercise the retry path in [`Indexer::publish_file`](crate::Indexer::publish_file).
    transient_failures_remaining: Mutex<u32>,
}

impl MockStorage {
    pub fn fail_next(&self, n: u32) {
        // Synchronous setter for test setup convenience. Acquire via try_lock; tests are
        // single-threaded relative to this counter so contention is impossible.
        *self
            .transient_failures_remaining
            .try_lock()
            .expect("uncontended") = n;
    }

    /// Generate a deterministic-looking CID from bytes. Real Codex CIDs are hashes; we mimic
    /// the multibase 'z' prefix and base58btc tail so [`looks_like_cid`](registry_core::looks_like_cid)
    /// accepts them.
    fn synthesize_cid(bytes: &[u8]) -> Cid {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"mock-codex:");
        h.update(bytes);
        let digest = h.finalize();
        format!("z{}", bs58::encode(&digest[..]).into_string())
    }
}

#[async_trait]
impl StorageClient for MockStorage {
    async fn upload(&self, bytes: &[u8]) -> Result<Cid, StorageError> {
        let mut remaining = self.transient_failures_remaining.lock().await;
        if *remaining > 0 {
            *remaining -= 1;
            return Err(StorageError::Transient("mock injected failure".into()));
        }
        drop(remaining);

        let cid = Self::synthesize_cid(bytes);
        self.inner.write().await.insert(cid.clone(), bytes.to_vec());
        Ok(cid)
    }

    async fn download(&self, cid: &Cid) -> Result<Vec<u8>, StorageError> {
        self.inner
            .read()
            .await
            .get(cid)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(cid.clone()))
    }
}

#[derive(Default)]
pub struct MockDelivery {
    senders: Mutex<HashMap<String, Vec<mpsc::UnboundedSender<Envelope>>>>,
}

#[async_trait]
impl DeliveryClient for MockDelivery {
    async fn publish(&self, topic: &str, envelope: &Envelope) -> Result<(), DeliveryError> {
        let mut senders = self.senders.lock().await;
        let entry = senders.entry(topic.to_string()).or_default();
        entry.retain(|s| s.send(envelope.clone()).is_ok());
        Ok(())
    }

    async fn subscribe(
        &self,
        topic: &str,
    ) -> Result<mpsc::UnboundedReceiver<Envelope>, DeliveryError> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.senders
            .lock()
            .await
            .entry(topic.to_string())
            .or_default()
            .push(tx);
        Ok(rx)
    }
}

#[derive(Default)]
pub struct MockAnchor {
    entries: RwLock<HashMap<String, RegistryEntry>>,
    tx_counter: Mutex<u64>,
}

#[async_trait]
impl AnchorClient for MockAnchor {
    async fn submit_batch(&self, entries: Vec<EntryRequest>) -> Result<TxHash, AnchorError> {
        if entries.is_empty() {
            return Err(AnchorError::InvalidBatch("empty batch".into()));
        }
        if entries.len() > registry_core::MAX_BATCH_ENTRIES {
            return Err(AnchorError::InvalidBatch(format!(
                "batch size {} exceeds max {}",
                entries.len(),
                registry_core::MAX_BATCH_ENTRIES
            )));
        }

        let now = chrono::Utc::now().timestamp() as u64;
        let mut store = self.entries.write().await;
        for req in entries {
            // Idempotency: skip CIDs already anchored, matching the on-chain program's behavior.
            store.entry(req.cid.clone()).or_insert(RegistryEntry {
                cid: req.cid,
                metadata_hash: req.metadata_hash,
                anchor_timestamp: now,
            });
        }

        let mut counter = self.tx_counter.lock().await;
        *counter += 1;
        Ok(format!("mock-tx-{:016x}", *counter))
    }

    async fn lookup(&self, cid: &str) -> Result<Option<RegistryEntry>, AnchorError> {
        Ok(self.entries.read().await.get(cid).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn storage_roundtrip() {
        let s = storage();
        let cid = s.upload(b"hello").await.unwrap();
        assert!(cid.starts_with('z'));
        let back = s.download(&cid).await.unwrap();
        assert_eq!(back, b"hello");
    }

    #[tokio::test]
    async fn storage_injected_failure_surfaces_as_transient() {
        let s = storage();
        s.fail_next(1);
        let err = s.upload(b"x").await.unwrap_err();
        assert!(err.is_transient());
        // Subsequent call succeeds.
        let cid = s.upload(b"x").await.unwrap();
        assert!(cid.starts_with('z'));
    }

    #[tokio::test]
    async fn delivery_publish_reaches_subscriber() {
        let d = delivery();
        let mut rx = d.subscribe("/topic/1").await.unwrap();
        let env = sample_envelope("abc");
        d.publish("/topic/1", &env).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.cid, "abc");
    }

    #[tokio::test]
    async fn delivery_does_not_cross_topics() {
        let d = delivery();
        let mut rx_other = d.subscribe("/topic/other").await.unwrap();
        d.publish("/topic/main", &sample_envelope("x"))
            .await
            .unwrap();
        // No message on the other topic.
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(50), rx_other.recv()).await;
        assert!(
            result.is_err(),
            "should time out — no message on this topic"
        );
    }

    #[tokio::test]
    async fn anchor_is_idempotent() {
        let a = anchor();
        let entry = EntryRequest {
            cid: "zABC".into(),
            metadata_hash: [1u8; 32],
        };
        a.submit_batch(vec![entry.clone()]).await.unwrap();
        let ts1 = a.lookup("zABC").await.unwrap().unwrap().anchor_timestamp;
        // Re-anchoring the same CID must not change the timestamp.
        a.submit_batch(vec![entry]).await.unwrap();
        let ts2 = a.lookup("zABC").await.unwrap().unwrap().anchor_timestamp;
        assert_eq!(ts1, ts2);
    }

    #[tokio::test]
    async fn anchor_rejects_empty_batch() {
        let err = anchor().submit_batch(vec![]).await.unwrap_err();
        assert!(matches!(err, AnchorError::InvalidBatch(_)));
    }

    #[tokio::test]
    async fn anchor_rejects_oversized_batch() {
        let huge = (0..(registry_core::MAX_BATCH_ENTRIES + 1))
            .map(|i| EntryRequest {
                cid: format!("z{}", i),
                metadata_hash: [0u8; 32],
            })
            .collect();
        let err = anchor().submit_batch(huge).await.unwrap_err();
        assert!(matches!(err, AnchorError::InvalidBatch(_)));
    }

    fn sample_envelope(cid: &str) -> Envelope {
        Envelope {
            cid: cid.into(),
            title: "t".into(),
            description: "d".into(),
            content_type: "text/plain".into(),
            size_bytes: 1,
            timestamp: 0,
            tags: vec![],
        }
    }
}

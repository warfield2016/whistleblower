//! End-to-end integration test: publisher → broadcast → batch-anchor watcher → lookup.
//!
//! Wires together the Indexer with shared mock backends to prove the full pipeline works.
//! This is the test that demonstrates the prize spec's "upload → broadcast → batch anchor"
//! flow without any real Logos infrastructure, and is the closest analog we have in CI to
//! `scripts/demo.sh` against a real sequencer.

use std::sync::Arc;
use std::time::Duration;

use doc_index_core::{clients::mock, Indexer, PublishRequest};
use registry_core::{metadata_hash, EntryRequest};

fn req(title: &str) -> PublishRequest {
    PublishRequest {
        title: title.into(),
        description: "integration test".into(),
        content_type: "text/plain".into(),
        tags: vec!["integration".into()],
        broadcast: true,
    }
}

#[tokio::test]
async fn end_to_end_single_publish_then_third_party_anchor() {
    // Shared backends — publisher and watcher operate on the same world.
    let storage = mock::storage();
    let delivery = mock::delivery();
    let anchor = mock::anchor();

    let publisher = Indexer::new(storage.clone(), delivery.clone(), anchor.clone());
    // The watcher (third-party anchor agent) doesn't even need access to storage —
    // it only listens on the delivery topic and writes to the anchor registry.
    let watcher = Arc::new(Indexer::new(
        mock::storage(),
        delivery.clone(),
        anchor.clone(),
    ));

    let mut rx = watcher.subscribe().await.unwrap();

    let receipt = publisher
        .publish_file(b"secret memo about Q3 budget", req("Q3 leak"))
        .await
        .expect("publish");

    let envelope = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("delivery received in time")
        .expect("channel still open");
    assert_eq!(envelope.cid, receipt.cid);

    // Third party anchors what they saw, no coordination with publisher.
    let hash = metadata_hash(&envelope);
    let anchor_receipt = watcher
        .anchor_batch(vec![EntryRequest {
            cid: envelope.cid.clone(),
            metadata_hash: hash,
        }])
        .await
        .unwrap();
    assert_eq!(anchor_receipt.anchored_cids.len(), 1);

    // Publisher queries the registry and sees their CID anchored.
    let entry = publisher.lookup(&receipt.cid).await.unwrap().unwrap();
    assert_eq!(entry.cid, receipt.cid);
    assert_eq!(entry.metadata_hash, hash);
}

#[tokio::test]
async fn end_to_end_batch_anchor_groups_many_publishes() {
    let storage = mock::storage();
    let delivery = mock::delivery();
    let anchor = mock::anchor();

    let publisher = Indexer::new(storage, delivery.clone(), anchor.clone());
    let watcher = Indexer::new(mock::storage(), delivery.clone(), anchor.clone());

    let mut rx = watcher.subscribe().await.unwrap();

    // Publisher emits 15 documents.
    let mut expected_cids = Vec::new();
    for i in 0..15 {
        let body = format!("document {} contents", i);
        let r = publisher
            .publish_file(body.as_bytes(), req(&format!("doc-{:02}", i)))
            .await
            .unwrap();
        expected_cids.push(r.cid);
    }

    // Watcher accumulates the envelopes.
    let mut batch = Vec::with_capacity(15);
    for _ in 0..15 {
        let envelope = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        batch.push(EntryRequest {
            cid: envelope.cid.clone(),
            metadata_hash: metadata_hash(&envelope),
        });
    }

    // Single batch anchor.
    let receipt = watcher.anchor_batch(batch).await.unwrap();
    assert_eq!(receipt.anchored_cids.len(), 15);
    assert_eq!(receipt.skipped_duplicate_cids.len(), 0);

    // All 15 are queryable.
    for cid in &expected_cids {
        let entry = publisher.lookup(cid).await.unwrap();
        assert!(entry.is_some(), "missing entry for {}", cid);
    }
}

#[tokio::test]
async fn end_to_end_resubmission_is_idempotent() {
    let storage = mock::storage();
    let delivery = mock::delivery();
    let anchor = mock::anchor();

    let publisher = Indexer::new(storage, delivery.clone(), anchor.clone());
    let watcher = Indexer::new(mock::storage(), delivery, anchor);

    let mut rx = watcher.subscribe().await.unwrap();
    let receipt = publisher.publish_file(b"data", req("d")).await.unwrap();

    let envelope = rx.recv().await.unwrap();
    let entry = EntryRequest {
        cid: envelope.cid.clone(),
        metadata_hash: metadata_hash(&envelope),
    };

    let first = watcher.anchor_batch(vec![entry.clone()]).await.unwrap();
    assert_eq!(first.anchored_cids.len(), 1);

    // Re-submitting the same CID does not create a duplicate.
    let second = watcher.anchor_batch(vec![entry]).await.unwrap();
    assert_eq!(second.anchored_cids.len(), 0);
    assert_eq!(second.skipped_duplicate_cids, vec![receipt.cid]);
}

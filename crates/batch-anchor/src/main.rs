//! `batch-anchor` — permissionless batch anchor daemon.
//!
//! Subscribes to a Waku topic, accumulates envelopes, and anchors them to chronicle-registry
//! in batches. **Anyone can run this** — no shared keys, no coordination with publishers.
//!
//! Idempotency state lives in a local SQLite file: we track CIDs we've already submitted so
//! a restart after partial flush doesn't re-submit, and we track the last successful flush
//! timestamp so we can resume cleanly.
//!
//! In the scaffold this drives the mock delivery/anchor for end-to-end testing. The same
//! binary runs against real Logos once `doc-index-core`'s `real-logos` clients are wired up.

mod state;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use doc_index_core::{clients::mock, Indexer};
use registry_core::{metadata_hash, EntryRequest, DEFAULT_WAKU_TOPIC, MAX_BATCH_ENTRIES};
use tracing::{debug, info, warn};

#[derive(Parser, Debug)]
#[command(
    name = "batch-anchor",
    version,
    about = "Anchor broadcast CIDs to chronicle-registry in batches"
)]
struct Cli {
    /// SQLite file for idempotency state. Created if missing.
    #[arg(long, default_value = "./batch-anchor.db")]
    state_file: PathBuf,

    /// Waku topic to subscribe to.
    #[arg(long, default_value = DEFAULT_WAKU_TOPIC)]
    topic: String,

    /// Flush the buffer when this many entries accumulate.
    #[arg(long, default_value_t = MAX_BATCH_ENTRIES)]
    flush_size: usize,

    /// Flush the buffer at least this often (seconds), even if undersized.
    #[arg(long, default_value_t = 30)]
    flush_interval_secs: u64,

    /// Run for a fixed duration (seconds) and exit. 0 = forever. Useful for the demo script.
    #[arg(long, default_value_t = 0)]
    run_for_secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let state = state::State::open(&cli.state_file)
        .with_context(|| format!("opening state at {}", cli.state_file.display()))?;
    info!(
        path = %cli.state_file.display(),
        last_flush = ?state.last_flush_timestamp(),
        seen_count = state.seen_count(),
        "opened state"
    );

    // In production these would be the real Codex / Waku / SPEL clients.
    // The scaffold uses mocks so the binary is runnable end-to-end today.
    let indexer = Arc::new(Indexer::new(
        mock::storage(),
        mock::delivery(),
        mock::anchor(),
    ));

    run(indexer, state, cli).await
}

async fn run(indexer: Arc<Indexer>, mut state: state::State, cli: Cli) -> Result<()> {
    let mut rx = indexer.subscribe().await?;
    let mut buffer: Vec<EntryRequest> = Vec::with_capacity(cli.flush_size);
    let mut interval = tokio::time::interval(Duration::from_secs(cli.flush_interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let deadline = if cli.run_for_secs > 0 {
        Some(tokio::time::Instant::now() + Duration::from_secs(cli.run_for_secs))
    } else {
        None
    };

    loop {
        // Select between: new envelope arrives, periodic flush tick, or run-for deadline.
        tokio::select! {
            biased;

            _ = sleep_until_or_pending(deadline) => {
                info!("run-for deadline reached, flushing and exiting");
                if !buffer.is_empty() {
                    flush(&indexer, &mut state, &mut buffer).await?;
                }
                break;
            }

            maybe_envelope = rx.recv() => {
                match maybe_envelope {
                    Some(envelope) => {
                        if state.has_seen(&envelope.cid) {
                            debug!(cid = %envelope.cid, "already seen, skipping");
                            continue;
                        }
                        let hash = metadata_hash(&envelope);
                        buffer.push(EntryRequest {
                            cid: envelope.cid.clone(),
                            metadata_hash: hash,
                        });
                        state.mark_seen(&envelope.cid)?;
                        debug!(cid = %envelope.cid, buf = buffer.len(), "buffered");

                        if buffer.len() >= cli.flush_size {
                            flush(&indexer, &mut state, &mut buffer).await?;
                        }
                    }
                    None => {
                        warn!("subscription channel closed; flushing and exiting");
                        if !buffer.is_empty() {
                            flush(&indexer, &mut state, &mut buffer).await?;
                        }
                        break;
                    }
                }
            }

            _ = interval.tick() => {
                if !buffer.is_empty() {
                    flush(&indexer, &mut state, &mut buffer).await?;
                }
            }
        }
    }

    Ok(())
}

async fn flush(
    indexer: &Indexer,
    state: &mut state::State,
    buffer: &mut Vec<EntryRequest>,
) -> Result<()> {
    let batch: Vec<EntryRequest> = std::mem::take(buffer);
    let count = batch.len();
    info!(count, "flushing batch to chronicle-registry");

    match indexer.anchor_batch(batch.clone()).await {
        Ok(r) => {
            info!(
                tx = %r.tx_hash,
                anchored = r.anchored_cids.len(),
                skipped = r.skipped_duplicate_cids.len(),
                "batch anchored"
            );
            state.record_flush(chrono::Utc::now().timestamp())?;
        }
        Err(e) => {
            warn!(error = %e, "flush failed — restoring buffer for retry on next tick");
            *buffer = batch;
        }
    }
    Ok(())
}

async fn sleep_until_or_pending(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(d) => tokio::time::sleep_until(d).await,
        None => std::future::pending::<()>().await,
    }
}

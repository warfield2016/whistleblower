//! `doc-index` — CLI wrapper around the doc-index-core module.
//!
//! Useful for smoke-testing the full pipeline outside the Basecamp app and as the
//! publisher half of the demo script. Anchors here go through the mock anchor by
//! default; pass `--real` to use the real SPEL CLI shell-out (not yet implemented).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use doc_index_core::{clients::mock, Indexer, PublishRequest};
use registry_core::EntryRequest;

#[derive(Parser, Debug)]
#[command(
    name = "doc-index",
    version,
    about = "Publish, anchor, and query documents on the Logos stack"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// Waku topic (default: /whistleblower/1/document-index/borsh)
    #[arg(long, global = true)]
    topic: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Upload a file and broadcast its envelope.
    Publish {
        /// Path to the file to publish.
        file: PathBuf,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, default_value = "application/octet-stream")]
        content_type: String,
        #[arg(long)]
        tags: Vec<String>,
        /// Skip the broadcast and only upload.
        #[arg(long)]
        no_broadcast: bool,
        /// Also anchor on-chain immediately after publishing.
        #[arg(long)]
        anchor: bool,
    },
    /// Anchor a single CID (or a list of them) on-chain.
    Anchor {
        /// CID(s) to anchor.
        #[arg(required = true)]
        cids: Vec<String>,
        /// Wire-format metadata hash per CID (v1:<hex>). Must match count of cids.
        #[arg(long, required = true)]
        metadata_hash: Vec<String>,
    },
    /// Look up a CID in the on-chain registry.
    Lookup { cid: String },
    /// List documents published in this process.
    List,
    /// Run publish → anchor → lookup in a single process. Used by demo.sh to exercise the
    /// full pipeline through mock backends (separate CLI invocations would each get fresh
    /// mocks, which is unrepresentative of how real Logos backends share state).
    Demo {
        /// Path to the file to publish.
        file: PathBuf,
        #[arg(long, default_value = "Demo document")]
        title: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let mut indexer = Indexer::new(mock::storage(), mock::delivery(), mock::anchor());
    if let Some(t) = cli.topic {
        indexer = indexer.with_topic(t);
    }

    match cli.cmd {
        Cmd::Publish {
            file,
            title,
            description,
            content_type,
            tags,
            no_broadcast,
            anchor,
        } => {
            let bytes = fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let receipt = indexer
                .publish_file(
                    &bytes,
                    PublishRequest {
                        title,
                        description,
                        content_type,
                        tags,
                        broadcast: !no_broadcast,
                    },
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);

            if anchor {
                let hash = registry_core::parse_metadata_hash(&receipt.metadata_hash)
                    .context("hash should round-trip")?;
                let ar = indexer
                    .anchor_batch(vec![EntryRequest {
                        cid: receipt.cid,
                        metadata_hash: hash,
                    }])
                    .await?;
                println!("{}", serde_json::to_string_pretty(&ar)?);
            }
        }
        Cmd::Anchor {
            cids,
            metadata_hash,
        } => {
            anyhow::ensure!(
                cids.len() == metadata_hash.len(),
                "--metadata-hash count ({}) must match cid count ({})",
                metadata_hash.len(),
                cids.len()
            );
            let entries: Vec<EntryRequest> = cids
                .into_iter()
                .zip(metadata_hash.into_iter())
                .map(|(cid, hash_str)| {
                    let hash = registry_core::parse_metadata_hash(&hash_str)
                        .with_context(|| format!("malformed metadata hash: {}", hash_str))?;
                    Ok::<_, anyhow::Error>(EntryRequest {
                        cid,
                        metadata_hash: hash,
                    })
                })
                .collect::<Result<_>>()?;
            let r = indexer.anchor_batch(entries).await?;
            println!("{}", serde_json::to_string_pretty(&r)?);
        }
        Cmd::Lookup { cid } => {
            let entry = indexer.lookup(&cid).await?;
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        Cmd::List => {
            let records = indexer.list_published().await;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        Cmd::Demo { file, title } => {
            let bytes = fs::read(&file).with_context(|| format!("reading {}", file.display()))?;

            println!("[demo 1/3] publish");
            let receipt = indexer
                .publish_file(
                    &bytes,
                    PublishRequest {
                        title,
                        description: "single-process demo".into(),
                        content_type: "application/octet-stream".into(),
                        tags: vec!["demo".into()],
                        broadcast: true,
                    },
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);

            println!("\n[demo 2/3] anchor");
            let hash = registry_core::parse_metadata_hash(&receipt.metadata_hash)
                .context("hash should round-trip")?;
            let anchor_receipt = indexer
                .anchor_batch(vec![EntryRequest {
                    cid: receipt.cid.clone(),
                    metadata_hash: hash,
                }])
                .await?;
            println!("{}", serde_json::to_string_pretty(&anchor_receipt)?);

            println!("\n[demo 3/3] lookup");
            let entry = indexer.lookup(&receipt.cid).await?;
            match &entry {
                Some(_) => println!("{}", serde_json::to_string_pretty(&entry)?),
                None => anyhow::bail!("lookup returned null after anchor — this is a bug"),
            }
            println!(
                "\n[demo done] CID {} is anchored and queryable.",
                receipt.cid
            );
        }
    }

    Ok(())
}

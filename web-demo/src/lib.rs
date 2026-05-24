//! WASM bindings for the doc-index pipeline.
//!
//! Re-implements the upload → broadcast → anchor flow synchronously, in-process, using
//! the same wire types as the production [`registry-core`] crate. The browser demo at
//! `/web/` calls these bindings from React; the behaviour mirrors the Rust mocks in
//! `doc-index-core::clients::mock` so the UX evaluators see is the same UX a Basecamp
//! app gets from the real `logos.module("doc-index")` bridge.
//!
//! ## Why a separate crate?
//!
//! `doc-index-core` uses `tokio` for async orchestration, which doesn't fully work in
//! browser WASM (no threads, no I/O). Rather than ifdef the production crate to half-
//! support WASM, we keep concerns separate: this crate is the browser demo surface, the
//! production crate is the server / Basecamp surface, and both speak the same wire types.

use std::cell::RefCell;
use std::collections::HashMap;

use registry_core::{
    format_metadata_hash, looks_like_cid, metadata_hash, parse_metadata_hash, Envelope,
    EntryRequest, RegistryEntry, DEFAULT_WAKU_TOPIC, MAX_BATCH_ENTRIES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

/// Install a panic hook on load so Rust panics show up in the browser console as readable
/// stack traces instead of generic "unreachable executed" errors.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// --- Pipeline state ---
//
// Single in-process state — the WASM module is loaded once per page, so a thread-local
// RefCell is the simplest way to keep mutable state across calls. No threads in browser
// WASM means no need for Arc/Mutex.

thread_local! {
    static STATE: RefCell<DemoState> = RefCell::new(DemoState::default());
}

#[derive(Default)]
struct DemoState {
    /// Maps CID → uploaded bytes (the "Codex storage" mock).
    storage: HashMap<String, Vec<u8>>,
    /// CIDs we've broadcast in this session — used for the "deduplicated" guarantee.
    broadcast_seen: HashMap<String, bool>,
    /// On-chain registry: CID → RegistryEntry. Mirrors the LEZ chronicle-registry state.
    anchored: HashMap<String, RegistryEntry>,
    /// Envelopes received on the topic, kept in order so the watcher view can replay them.
    delivery_log: Vec<Envelope>,
    /// Local "my published documents" list, like the Indexer's PublishedRecord store.
    published: Vec<PublishedRecord>,
    /// Monotonic tx counter for synthetic transaction hashes.
    tx_counter: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct PublishedRecord {
    publish_id: String,
    envelope: Envelope,
    metadata_hash: String,
    anchored: bool,
    anchor_tx: Option<String>,
}

// --- Helpers ---

fn synth_cid(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"web-demo-codex:");
    h.update(bytes);
    let digest = h.finalize();
    format!("z{}", bs58::encode(&digest[..]).into_string())
}

fn now_ts() -> u64 {
    let ms = js_sys::Date::now();
    (ms / 1000.0) as u64
}

/// Serialize to a JsValue using the JSON-compatible mode.
///
/// Default serde-wasm-bindgen produces JS Map objects, but JS callers expect
/// plain `{ok: true, ...}` records. The `json_compatible()` serializer matches
/// what `JSON.parse(JSON.stringify(...))` would give you — which is what every
/// React consumer in `web/app/page.tsx` is written against.
fn to_js<T: Serialize + ?Sized>(value: &T) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&serializer)
        .expect("response always serializes")
}

fn ok<T: Serialize>(value: T) -> JsValue {
    #[derive(Serialize)]
    struct Wrapper<T: Serialize> {
        ok: bool,
        #[serde(flatten)]
        inner: T,
    }
    to_js(&Wrapper { ok: true, inner: value })
}

fn err(msg: impl Into<String>) -> JsValue {
    let msg = msg.into();
    to_js(&serde_json::json!({ "ok": false, "error": msg }))
}

// --- Public WASM API ---
//
// Names mirror the Rust C ABI exports in `doc-index-core/src/ffi.rs`. JSON shapes match
// `docs/API.md`. The only departure: bytes for `publishFile` come in as a Uint8Array
// directly (not via a pointer), which is the idiomatic wasm-bindgen pattern.

#[derive(Deserialize)]
struct PublishRequest {
    title: String,
    #[serde(default)]
    description: String,
    content_type: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_true")]
    broadcast: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
struct PublishReceipt {
    cid: String,
    publish_id: String,
    metadata_hash: String,
    broadcast: bool,
    timestamp: u64,
}

/// publishFileJson(requestJson: string, bytes: Uint8Array) → JSON
///
/// Uploads bytes, builds envelope, optionally broadcasts on the delivery topic.
/// Mirrors `doc_index_publish_file_json` in the production FFI.
#[wasm_bindgen(js_name = publishFileJson)]
pub fn publish_file_json(request_json: &str, bytes: &[u8]) -> JsValue {
    let req: PublishRequest = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => return err(format!("malformed request: {}", e)),
    };
    if req.title.is_empty() {
        return err("title must not be empty");
    }
    if req.content_type.is_empty() {
        return err("content_type must not be empty");
    }

    let cid = synth_cid(bytes);
    let envelope = Envelope {
        cid: cid.clone(),
        title: req.title,
        description: req.description,
        content_type: req.content_type,
        size_bytes: bytes.len() as u64,
        timestamp: now_ts(),
        tags: req.tags,
    };
    let hash = metadata_hash(&envelope);
    let hash_wire = format_metadata_hash(&hash);
    let publish_id = uuid::Uuid::new_v4().to_string();

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.storage.insert(cid.clone(), bytes.to_vec());

        if req.broadcast {
            // Deduplicate by CID, matching the prize's "broadcast deduplicated" criterion.
            if !state.broadcast_seen.contains_key(&cid) {
                state.broadcast_seen.insert(cid.clone(), true);
                state.delivery_log.push(envelope.clone());
            }
        }

        state.published.push(PublishedRecord {
            publish_id: publish_id.clone(),
            envelope: envelope.clone(),
            metadata_hash: hash_wire.clone(),
            anchored: false,
            anchor_tx: None,
        });
    });

    ok(PublishReceipt {
        cid,
        publish_id,
        metadata_hash: hash_wire,
        broadcast: req.broadcast,
        timestamp: envelope.timestamp,
    })
}

#[derive(Deserialize)]
struct AnchorBatchRequest {
    entries: Vec<EntryWire>,
}

#[derive(Deserialize)]
struct EntryWire {
    cid: String,
    metadata_hash: String,
}

#[derive(Serialize)]
struct AnchorReceipt {
    tx_hash: String,
    anchored_cids: Vec<String>,
    skipped_duplicate_cids: Vec<String>,
}

/// anchorBatchJson({entries: [{cid, metadata_hash}]}) → JSON
#[wasm_bindgen(js_name = anchorBatchJson)]
pub fn anchor_batch_json(request_json: &str) -> JsValue {
    let req: AnchorBatchRequest = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => return err(format!("malformed request: {}", e)),
    };
    if req.entries.is_empty() {
        return err("batch must not be empty");
    }
    if req.entries.len() > MAX_BATCH_ENTRIES {
        return err(format!(
            "batch size {} exceeds max {}",
            req.entries.len(),
            MAX_BATCH_ENTRIES
        ));
    }

    let mut parsed = Vec::with_capacity(req.entries.len());
    for wire in req.entries {
        if !looks_like_cid(&wire.cid) {
            return err(format!("invalid CID: {}", wire.cid));
        }
        let hash = match parse_metadata_hash(&wire.metadata_hash) {
            Some(h) => h,
            None => return err(format!("malformed metadata_hash: {}", wire.metadata_hash)),
        };
        parsed.push(EntryRequest {
            cid: wire.cid,
            metadata_hash: hash,
        });
    }

    let receipt = STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.tx_counter += 1;
        let tx_hash = format!("demo-tx-{:016x}", state.tx_counter);
        let ts = now_ts();
        let mut anchored = Vec::new();
        let mut skipped = Vec::new();
        for entry in parsed {
            if state.anchored.contains_key(&entry.cid) {
                skipped.push(entry.cid);
            } else {
                state.anchored.insert(
                    entry.cid.clone(),
                    RegistryEntry {
                        cid: entry.cid.clone(),
                        metadata_hash: entry.metadata_hash,
                        anchor_timestamp: ts,
                    },
                );
                anchored.push(entry.cid);
            }
        }
        for record in state.published.iter_mut() {
            if anchored.contains(&record.envelope.cid) {
                record.anchored = true;
                record.anchor_tx = Some(tx_hash.clone());
            }
        }
        AnchorReceipt {
            tx_hash,
            anchored_cids: anchored,
            skipped_duplicate_cids: skipped,
        }
    });

    ok(receipt)
}

#[derive(Deserialize)]
struct LookupRequest {
    cid: String,
}

#[derive(Serialize)]
struct LookupResponse {
    entry: Option<RegistryEntry>,
}

/// lookupJson({"cid":"..."}) → JSON
#[wasm_bindgen(js_name = lookupJson)]
pub fn lookup_json(request_json: &str) -> JsValue {
    let req: LookupRequest = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => return err(format!("malformed request: {}", e)),
    };
    let entry = STATE.with(|s| s.borrow().anchored.get(&req.cid).cloned());
    ok(LookupResponse { entry })
}

/// listPublishedJson() → JSON array of PublishedRecord
#[wasm_bindgen(js_name = listPublishedJson)]
pub fn list_published_json() -> JsValue {
    let records: Vec<PublishedRecord> = STATE.with(|s| s.borrow().published.clone());
    to_js(&records)
}

/// listDeliveryLogJson() → JSON array of Envelope (the broadcast topic, in arrival order)
///
/// Lets the UI show "what a third-party watcher would see on Waku". Not in the production
/// FFI surface — added for the demo's "third-party anchor" visualization.
#[wasm_bindgen(js_name = listDeliveryLogJson)]
pub fn list_delivery_log_json() -> JsValue {
    let envelopes: Vec<Envelope> = STATE.with(|s| s.borrow().delivery_log.clone());
    to_js(&envelopes)
}

/// listAnchoredJson() → JSON array of RegistryEntry (the on-chain registry contents)
#[wasm_bindgen(js_name = listAnchoredJson)]
pub fn list_anchored_json() -> JsValue {
    let entries: Vec<RegistryEntry> = STATE.with(|s| s.borrow().anchored.values().cloned().collect());
    to_js(&entries)
}

/// resetDemoState() — clear all in-memory state. Useful between demo runs.
#[wasm_bindgen(js_name = resetDemoState)]
pub fn reset_demo_state() {
    STATE.with(|s| *s.borrow_mut() = DemoState::default());
}

/// getTopic() → the default Waku topic the production module broadcasts on.
#[wasm_bindgen(js_name = getTopic)]
pub fn get_topic() -> String {
    DEFAULT_WAKU_TOPIC.to_string()
}

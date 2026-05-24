//! C ABI bridge for use from QML (via `logos.module("doc-index").method(JSON)`) or any FFI host.
//!
//! Methods are JSON-in / JSON-out — matches the shape Basecamp modules expose via
//! `LogosAPIClient.invokeRemoteMethod`. Schemas mirror the reference chronicle module so a
//! Basecamp app can swap between implementations without touching its QML/JS layer.
//!
//! ## Scaffold status
//!
//! The functions defined here construct an [`Indexer`] from mocks at startup; the production
//! wiring (real CodexClient + WakuClient + SPEL anchor client) lives in
//! [`crate::clients::real`], gated on the `real-logos` feature, and is wired in once the
//! integration phase replaces the stubs there.
//!
//! ## Why pointer-based handles
//!
//! QML can't pass Rust Arc<Indexer> across the FFI boundary, so we hand out opaque `IndexerHandle`
//! integers backed by a process-wide registry. Same pattern as logos-agent's agent-core ffi.

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::clients::mock;
use crate::indexer::{Indexer, PublishRequest};
use registry_core::EntryRequest;

/// Opaque handle returned to FFI callers.
pub type IndexerHandle = u64;

struct HandleRegistry {
    next_id: u64,
    handles: HashMap<IndexerHandle, Arc<Indexer>>,
    runtime: Arc<tokio::runtime::Runtime>,
}

fn registry() -> &'static Mutex<HandleRegistry> {
    static REGISTRY: OnceLock<Mutex<HandleRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Mutex::new(HandleRegistry {
            next_id: 0,
            handles: HashMap::new(),
            runtime: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads(2)
                    .build()
                    .expect("tokio runtime"),
            ),
        })
    })
}

/// Create an Indexer wired to in-process mocks. Returns a handle for subsequent calls.
///
/// FFI callers in dev / preview / Basecamp's "developer mode" use this. Production callers
/// will use a `doc_index_new_real(config_json)` variant once the real backends are wired in.
#[unsafe(no_mangle)]
pub extern "C" fn doc_index_new_mock() -> IndexerHandle {
    let indexer = Arc::new(Indexer::new(
        mock::storage(),
        mock::delivery(),
        mock::anchor(),
    ));
    let mut reg = registry().lock().unwrap();
    reg.next_id += 1;
    let id = reg.next_id;
    reg.handles.insert(id, indexer);
    id
}

/// Free the handle. Subsequent calls with this handle return an error.
#[unsafe(no_mangle)]
pub extern "C" fn doc_index_free(handle: IndexerHandle) {
    registry().lock().unwrap().handles.remove(&handle);
}

/// publishFileJson({title, description, content_type, tags?, broadcast?}, bytes_ptr, bytes_len)
///
/// Returns a JSON string on success: `{"ok":true,"cid":"...","publish_id":"...","metadata_hash":"v1:..."}`
/// Returns a JSON error on failure: `{"ok":false,"error":"..."}`
///
/// Caller owns the returned C string and MUST free it via `doc_index_free_string`.
///
/// # Safety
/// `request_json` must be a NUL-terminated C string. `bytes_ptr` must be a valid pointer
/// to `bytes_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn doc_index_publish_file_json(
    handle: IndexerHandle,
    request_json: *const c_char,
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> *mut c_char {
    let indexer = match get_indexer(handle) {
        Some(i) => i,
        None => return error_json("invalid handle"),
    };

    let request_str = match cstr_to_str(request_json) {
        Ok(s) => s,
        Err(e) => return error_json(&format!("invalid request json: {}", e)),
    };

    let request: PublishRequest = match serde_json::from_str(request_str) {
        Ok(r) => r,
        Err(e) => return error_json(&format!("malformed request: {}", e)),
    };

    if bytes_ptr.is_null() {
        return error_json("bytes_ptr is null");
    }
    let bytes = unsafe { std::slice::from_raw_parts(bytes_ptr, bytes_len) };

    let rt = registry().lock().unwrap().runtime.clone();
    let result = rt.block_on(indexer.publish_file(bytes, request));

    match result {
        Ok(r) => ok_json(&r),
        Err(e) => error_json(&e.to_string()),
    }
}

/// anchorBatchJson({entries: [{cid, metadata_hash}, ...]})
///
/// # Safety
/// `request_json` must be a NUL-terminated valid UTF-8 C string. Caller owns the returned
/// pointer and must free it via `doc_index_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn doc_index_anchor_batch_json(
    handle: IndexerHandle,
    request_json: *const c_char,
) -> *mut c_char {
    #[derive(Deserialize)]
    struct Req {
        entries: Vec<EntryRequestWire>,
    }

    #[derive(Deserialize)]
    struct EntryRequestWire {
        cid: String,
        metadata_hash: String, // "v1:<hex>"
    }

    let indexer = match get_indexer(handle) {
        Some(i) => i,
        None => return error_json("invalid handle"),
    };
    let s = match cstr_to_str(request_json) {
        Ok(s) => s,
        Err(e) => return error_json(&format!("invalid json: {}", e)),
    };
    let req: Req = match serde_json::from_str(s) {
        Ok(r) => r,
        Err(e) => return error_json(&format!("malformed request: {}", e)),
    };

    let mut entries = Vec::with_capacity(req.entries.len());
    for wire in req.entries {
        match registry_core::parse_metadata_hash(&wire.metadata_hash) {
            Some(h) => entries.push(EntryRequest {
                cid: wire.cid,
                metadata_hash: h,
            }),
            None => {
                return error_json(&format!(
                    "malformed metadata_hash for cid {}: expected v1:<64-hex>",
                    wire.cid
                ))
            }
        }
    }

    let rt = registry().lock().unwrap().runtime.clone();
    match rt.block_on(indexer.anchor_batch(entries)) {
        Ok(r) => ok_json(&r),
        Err(e) => error_json(&e.to_string()),
    }
}

/// lookupJson({"cid":"..."})  →  {"ok":true,"entry":{...}|null}
///
/// # Safety
/// `request_json` must be a NUL-terminated valid UTF-8 C string. Caller owns the returned
/// pointer and must free it via `doc_index_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn doc_index_lookup_json(
    handle: IndexerHandle,
    request_json: *const c_char,
) -> *mut c_char {
    #[derive(Deserialize)]
    struct Req {
        cid: String,
    }
    #[derive(Serialize)]
    struct Resp {
        ok: bool,
        entry: Option<registry_core::RegistryEntry>,
    }

    let indexer = match get_indexer(handle) {
        Some(i) => i,
        None => return error_json("invalid handle"),
    };
    let s = match cstr_to_str(request_json) {
        Ok(s) => s,
        Err(e) => return error_json(&format!("invalid json: {}", e)),
    };
    let req: Req = match serde_json::from_str(s) {
        Ok(r) => r,
        Err(e) => return error_json(&format!("malformed request: {}", e)),
    };

    let rt = registry().lock().unwrap().runtime.clone();
    match rt.block_on(indexer.lookup(&req.cid)) {
        Ok(entry) => {
            let resp = Resp { ok: true, entry };
            CString::new(serde_json::to_string(&resp).unwrap())
                .unwrap()
                .into_raw()
        }
        Err(e) => error_json(&e.to_string()),
    }
}

/// listPublishedJson() → JSON array of PublishedRecord
#[unsafe(no_mangle)]
pub extern "C" fn doc_index_list_published_json(handle: IndexerHandle) -> *mut c_char {
    let indexer = match get_indexer(handle) {
        Some(i) => i,
        None => return error_json("invalid handle"),
    };
    let rt = registry().lock().unwrap().runtime.clone();
    let records = rt.block_on(indexer.list_published());
    CString::new(serde_json::to_string(&records).unwrap())
        .unwrap()
        .into_raw()
}

/// Free a string returned by any of the *_json functions.
///
/// # Safety
/// `s` must be a pointer previously returned by this crate. Calling twice is UB.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn doc_index_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    drop(unsafe { CString::from_raw(s) });
}

// --- helpers ---

fn get_indexer(handle: IndexerHandle) -> Option<Arc<Indexer>> {
    registry().lock().unwrap().handles.get(&handle).cloned()
}

unsafe fn cstr_to_str<'a>(p: *const c_char) -> Result<&'a str, &'static str> {
    if p.is_null() {
        return Err("null pointer");
    }
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map_err(|_| "non-UTF8 string")
}

fn ok_json<T: Serialize>(value: &T) -> *mut c_char {
    #[derive(Serialize)]
    struct Wrapper<'a, T: Serialize> {
        ok: bool,
        #[serde(flatten)]
        inner: &'a T,
    }
    let wrapped = Wrapper {
        ok: true,
        inner: value,
    };
    CString::new(serde_json::to_string(&wrapped).expect("serializable"))
        .expect("no NUL")
        .into_raw()
}

fn error_json(msg: &str) -> *mut c_char {
    let s = serde_json::json!({ "ok": false, "error": msg });
    CString::new(s.to_string()).unwrap().into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn round_trip<F: FnOnce(*const c_char) -> *mut c_char>(input: &str, f: F) -> String {
        let cstr = CString::new(input).unwrap();
        let out_ptr = f(cstr.as_ptr());
        assert!(!out_ptr.is_null());
        let out = unsafe { CStr::from_ptr(out_ptr) }
            .to_str()
            .unwrap()
            .to_string();
        unsafe { doc_index_free_string(out_ptr) };
        out
    }

    #[test]
    fn handle_lifecycle() {
        let h = doc_index_new_mock();
        assert!(h > 0);
        assert!(get_indexer(h).is_some());
        doc_index_free(h);
        assert!(get_indexer(h).is_none());
    }

    #[test]
    fn publish_and_lookup_through_ffi() {
        let h = doc_index_new_mock();

        let req =
            r#"{"title":"t","description":"d","content_type":"text/plain","broadcast":false}"#;
        let req_c = CString::new(req).unwrap();
        let bytes = b"hello";
        let resp_ptr =
            unsafe { doc_index_publish_file_json(h, req_c.as_ptr(), bytes.as_ptr(), bytes.len()) };
        let resp = unsafe { CStr::from_ptr(resp_ptr) }
            .to_str()
            .unwrap()
            .to_string();
        unsafe { doc_index_free_string(resp_ptr) };

        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["ok"], true);
        let cid = v["cid"].as_str().unwrap().to_string();
        let hash = v["metadata_hash"].as_str().unwrap().to_string();

        // Anchor
        let anchor_req = format!(
            r#"{{"entries":[{{"cid":"{}","metadata_hash":"{}"}}]}}"#,
            cid, hash
        );
        let anchor_resp = round_trip(&anchor_req, |p| unsafe {
            doc_index_anchor_batch_json(h, p)
        });
        let av: serde_json::Value = serde_json::from_str(&anchor_resp).unwrap();
        assert_eq!(av["ok"], true);

        // Lookup
        let lookup_req = format!(r#"{{"cid":"{}"}}"#, cid);
        let lookup_resp = round_trip(&lookup_req, |p| unsafe { doc_index_lookup_json(h, p) });
        let lv: serde_json::Value = serde_json::from_str(&lookup_resp).unwrap();
        assert_eq!(lv["ok"], true);
        assert_eq!(lv["entry"]["cid"], cid);

        doc_index_free(h);
    }

    #[test]
    fn errors_return_structured_json() {
        let h = doc_index_new_mock();
        let bad = round_trip("not json", |p| unsafe { doc_index_anchor_batch_json(h, p) });
        let v: serde_json::Value = serde_json::from_str(&bad).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["error"].is_string());
        doc_index_free(h);
    }
}

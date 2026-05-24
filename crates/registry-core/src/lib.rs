//! Shared types between the chronicle-registry LEZ program and off-chain clients.
//!
//! Lives in its own crate so the program and clients can both depend on it
//! without drift. The wire format defined here is what makes the system interoperable
//! across implementations — change it carefully.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum entries accepted in a single `IndexBatch` instruction.
/// Picked to stay well under the LEZ per-tx data limit while exceeding the prize-required minimum of 10.
pub const MAX_BATCH_ENTRIES: usize = 50;

/// Length of the canonical `metadata_hash` digest (sha256 raw bytes).
pub const METADATA_HASH_LEN: usize = 32;

/// Version prefix applied to `metadata_hash` strings on the wire.
/// Lets us evolve the canonicalisation rules without breaking existing anchors.
pub const METADATA_HASH_PREFIX: &str = "v1:";

/// Default Waku content topic for Whistleblower document broadcasts (LIP-23 format).
pub const DEFAULT_WAKU_TOPIC: &str = "/whistleblower/1/document-index/borsh";

/// One anchored document in the registry.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Codex content identifier, multibase-encoded (e.g. "zDvZRwzk...").
    pub cid: String,
    /// sha256 of the canonical envelope, raw bytes (no "v1:" prefix here).
    pub metadata_hash: [u8; METADATA_HASH_LEN],
    /// Unix seconds when the anchor transaction was processed.
    pub anchor_timestamp: u64,
}

/// Instruction set accepted by chronicle-registry.
///
/// Keep this tiny. Anything more elaborate (revocation, content classification, etc.)
/// belongs in a separate program — the registry's only job is "this CID existed at time T".
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum Instruction {
    /// Initialise the registry PDA. Called once.
    InitRegistry,
    /// Append entries to the registry. Idempotent: existing CIDs are skipped silently.
    IndexBatch { entries: Vec<EntryRequest> },
}

/// Caller-supplied entry. `anchor_timestamp` is filled in by the program at execution time
/// (we don't trust caller timestamps).
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct EntryRequest {
    pub cid: String,
    pub metadata_hash: [u8; METADATA_HASH_LEN],
}

/// Envelope broadcast on the Waku topic. The hash of its canonical form is what
/// gets anchored on-chain. Schema lifted from the reference Whistleblower implementation
/// so any client can subscribe and decode.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Envelope {
    pub cid: String,
    pub title: String,
    pub description: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Errors a chronicle-registry instruction can surface.
/// SPEL programs return `SpelError::custom(code, message)`; we expose the codes here so
/// off-chain clients can match on them programmatically.
pub mod error_codes {
    pub const E_BATCH_EMPTY: u32 = 1001;
    pub const E_BATCH_OVERSIZED: u32 = 1002;
    pub const E_REGISTRY_UNINITIALIZED: u32 = 1003;
    pub const E_REGISTRY_ALREADY_INITIALIZED: u32 = 1004;
    pub const E_UNAUTHORIZED_ANCHORER: u32 = 1005;
    pub const E_INVALID_CID: u32 = 1006;
}

/// Compute the canonical metadata hash for an envelope.
///
/// Canonicalisation here is "borsh serialization of the Envelope" rather than JSON canonical form.
/// Borsh is deterministic by construction, has no string-escaping ambiguity, and is the project
/// convention. The "v1:" prefix on the wire string lets us migrate later without ambiguity.
pub fn metadata_hash(envelope: &Envelope) -> [u8; METADATA_HASH_LEN] {
    let bytes = borsh::to_vec(envelope).expect("envelope is always serializable");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut out = [0u8; METADATA_HASH_LEN];
    out.copy_from_slice(&digest);
    out
}

/// Format a raw hash as the wire string "v1:<hex>".
pub fn format_metadata_hash(hash: &[u8; METADATA_HASH_LEN]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(METADATA_HASH_PREFIX.len() + METADATA_HASH_LEN * 2);
    s.push_str(METADATA_HASH_PREFIX);
    for b in hash {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

/// Parse the wire string back to raw bytes. Returns None on malformed input.
pub fn parse_metadata_hash(s: &str) -> Option<[u8; METADATA_HASH_LEN]> {
    let hex = s.strip_prefix(METADATA_HASH_PREFIX)?;
    if hex.len() != METADATA_HASH_LEN * 2 {
        return None;
    }
    let mut out = [0u8; METADATA_HASH_LEN];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = char_to_nibble(chunk[0])?;
        let lo = char_to_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn char_to_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Quick sanity check on Codex CIDs: must be non-empty, ASCII, reasonable length,
/// start with a multibase prefix we recognise, and have an alphanumeric body
/// (no whitespace, no punctuation). This is a cheap pre-anchor filter — it does
/// NOT validate that the CID actually resolves on Codex.
pub fn looks_like_cid(cid: &str) -> bool {
    if cid.len() < 10 || cid.len() > 128 {
        return false;
    }
    if !cid.is_ascii() {
        return false;
    }
    // Codex CID v1 multibase: 'z' (base58btc) is the common case. We accept a few neighbours.
    if !matches!(cid.as_bytes()[0], b'z' | b'b' | b'm' | b'k' | b'f') {
        return false;
    }
    // Body must be alphanumeric — covers the base58/base32/base64 charsets that CIDs use.
    cid.bytes().skip(1).all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_envelope() -> Envelope {
        Envelope {
            cid: "zDvZRwzkyHVgr59zFkX7vyfzK7oUP7Jc6k7qpFD9ssDi7V5fvdjw".into(),
            title: "leaked memo".into(),
            description: "Internal Q3 budget".into(),
            content_type: "application/pdf".into(),
            size_bytes: 12345,
            timestamp: 1_716_500_000,
            tags: vec!["whistleblower".into(), "finance".into()],
        }
    }

    #[test]
    fn metadata_hash_is_deterministic() {
        let env = sample_envelope();
        let h1 = metadata_hash(&env);
        let h2 = metadata_hash(&env);
        assert_eq!(h1, h2);
    }

    #[test]
    fn metadata_hash_changes_with_any_field() {
        let env = sample_envelope();
        let baseline = metadata_hash(&env);

        let mut env_title = env.clone();
        env_title.title = "different title".into();
        assert_ne!(baseline, metadata_hash(&env_title));

        let mut env_tags = env.clone();
        env_tags.tags.push("extra".into());
        assert_ne!(baseline, metadata_hash(&env_tags));

        let mut env_size = env;
        env_size.size_bytes += 1;
        assert_ne!(baseline, metadata_hash(&env_size));
    }

    #[test]
    fn metadata_hash_wire_roundtrip() {
        let h = metadata_hash(&sample_envelope());
        let s = format_metadata_hash(&h);
        assert!(s.starts_with("v1:"));
        assert_eq!(s.len(), 3 + 64);
        let parsed = parse_metadata_hash(&s).expect("roundtrip");
        assert_eq!(parsed, h);
    }

    #[test]
    fn parse_metadata_hash_rejects_garbage() {
        assert!(parse_metadata_hash("not a hash").is_none());
        assert!(parse_metadata_hash("v1:tooshort").is_none());
        assert!(parse_metadata_hash(
            "v2:0000000000000000000000000000000000000000000000000000000000000000"
        )
        .is_none());
        assert!(parse_metadata_hash(
            "v1:zz00000000000000000000000000000000000000000000000000000000000000"
        )
        .is_none());
    }

    #[test]
    fn instruction_roundtrips_through_borsh() {
        let ix = Instruction::IndexBatch {
            entries: vec![EntryRequest {
                cid: "zDv...".into(),
                metadata_hash: [7u8; METADATA_HASH_LEN],
            }],
        };
        let bytes = borsh::to_vec(&ix).unwrap();
        let parsed: Instruction = borsh::from_slice(&bytes).unwrap();
        assert_eq!(ix, parsed);
    }

    #[test]
    fn instruction_roundtrips_through_json() {
        let ix = Instruction::InitRegistry;
        let s = serde_json::to_string(&ix).unwrap();
        let parsed: Instruction = serde_json::from_str(&s).unwrap();
        assert_eq!(ix, parsed);
    }

    #[test]
    fn looks_like_cid_accepts_codex_format() {
        assert!(looks_like_cid(
            "zDvZRwzkyHVgr59zFkX7vyfzK7oUP7Jc6k7qpFD9ssDi7V5fvdjw"
        ));
        assert!(looks_like_cid("bafybeibwzpdxqg3mvrxnz5vdtmt"));
        assert!(!looks_like_cid(""));
        assert!(!looks_like_cid("not_a_multibase_prefix"));
        assert!(!looks_like_cid(&"z".repeat(200)));
    }
}

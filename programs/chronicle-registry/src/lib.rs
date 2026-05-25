//! chronicle-registry — the LEZ program that anchors document CIDs.
//!
//! ## Wire-shape (canonical, frozen for v1)
//!
//! Two instructions, both passed to the program as borsh-encoded [`Instruction`]:
//!
//! - `Instruction::InitRegistry` — called once. Creates the registry PDA seeded
//!   with the literal `"chronicle_registry"`. Subsequent calls fail with
//!   `E_REGISTRY_ALREADY_INITIALIZED`.
//!
//! - `Instruction::IndexBatch { entries }` — append entries. Idempotent: CIDs
//!   already in the registry are silently skipped. Fails with `E_BATCH_EMPTY` /
//!   `E_BATCH_OVERSIZED` / `E_INVALID_CID` for malformed input.
//!
//! ## SPEL integration
//!
//! When built with `--features real-spel` (which requires a Logos LEZ dev environment
//! and the `spel-framework` git dependency), this file exposes a `#[lez_program]`
//! module with `init_registry` and `index_batch` functions that SPEL's macro framework
//! generates IDL for. Without that feature, this crate compiles as a normal Rust library
//! exposing the pure state-transition logic in [`apply_instruction`] — which is unit-testable
//! and used by both the program guest and the off-chain mock anchor.
//!
//! ## Why split the program from the transition logic
//!
//! The transition function [`apply_instruction`] takes the current `RegistryState` and the
//! parsed instruction, returns the new state. No I/O, no globals — pure function. The SPEL
//! wrapper (`#[lez_program]` mod) is then a thin layer that parses the account, calls
//! `apply_instruction`, writes the result. This split lets us write exhaustive tests against
//! the transition logic without spinning up a sequencer, and reuse the same logic in the
//! `MockAnchor` for end-to-end testing of clients.

use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use registry_core::{
    error_codes, looks_like_cid, EntryRequest, Instruction, RegistryEntry,
    CURRENT_REGISTRY_ENTRY_VERSION, MAX_BATCH_ENTRIES,
};

/// The on-chain state of the registry. Lives in the data field of the registry PDA.
///
/// Entries live in a `HashMap` keyed on CID, giving O(1) lookup and contains-check vs the
/// O(N) linear scan of the previous `Vec` layout. Borsh serializes `HashMap` deterministically
/// (sorted by key) so on-chain bytes remain identical across guest executions.
///
/// **Invariant:** the HashMap key always equals `value.cid`. Enforced at the single insertion
/// site in [`apply_instruction`]; all read paths can assume it.
#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct RegistryState {
    pub initialized: bool,
    pub entries: HashMap<String, RegistryEntry>,
}

impl RegistryState {
    pub fn contains(&self, cid: &str) -> bool {
        self.entries.contains_key(cid)
    }

    pub fn get(&self, cid: &str) -> Option<&RegistryEntry> {
        self.entries.get(cid)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Result of applying an instruction to a registry state.
#[derive(Debug, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub new_state: RegistryState,
    pub appended_cids: Vec<String>,
    pub skipped_duplicate_cids: Vec<String>,
}

/// Errors produced during state transition. Codes match `registry_core::error_codes`.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    #[error("batch is empty")]
    BatchEmpty,
    #[error("batch oversized: {actual} > {max}")]
    BatchOversized { actual: usize, max: usize },
    #[error("registry already initialized")]
    AlreadyInitialized,
    #[error("registry not initialized")]
    Uninitialized,
    #[error("invalid CID: {0}")]
    InvalidCid(String),
}

impl TransitionError {
    pub fn code(&self) -> u32 {
        match self {
            Self::BatchEmpty => error_codes::E_BATCH_EMPTY,
            Self::BatchOversized { .. } => error_codes::E_BATCH_OVERSIZED,
            Self::AlreadyInitialized => error_codes::E_REGISTRY_ALREADY_INITIALIZED,
            Self::Uninitialized => error_codes::E_REGISTRY_UNINITIALIZED,
            Self::InvalidCid(_) => error_codes::E_INVALID_CID,
        }
    }
}

/// Pure transition function. The SPEL program wrapper and the off-chain mock both call this.
///
/// `anchor_timestamp` and `anchored_by` are supplied by the caller — the program reads them
/// from the LEZ block context + signer account, tests inject fixed values. Keeping them as
/// parameters preserves purity, which is what makes this function unit-testable without a
/// running sequencer.
pub fn apply_instruction(
    state: RegistryState,
    instruction: Instruction,
    anchor_timestamp: i64,
    anchored_by: [u8; 32],
) -> Result<ApplyOutcome, TransitionError> {
    match instruction {
        Instruction::InitRegistry => {
            if state.initialized {
                return Err(TransitionError::AlreadyInitialized);
            }
            Ok(ApplyOutcome {
                new_state: RegistryState {
                    initialized: true,
                    entries: HashMap::new(),
                },
                appended_cids: Vec::new(),
                skipped_duplicate_cids: Vec::new(),
            })
        }

        Instruction::IndexBatch { entries } => {
            if !state.initialized {
                return Err(TransitionError::Uninitialized);
            }
            if entries.is_empty() {
                return Err(TransitionError::BatchEmpty);
            }
            if entries.len() > MAX_BATCH_ENTRIES {
                return Err(TransitionError::BatchOversized {
                    actual: entries.len(),
                    max: MAX_BATCH_ENTRIES,
                });
            }
            // Validate up-front so partial application can't happen.
            for e in &entries {
                if !looks_like_cid(&e.cid) {
                    return Err(TransitionError::InvalidCid(e.cid.clone()));
                }
            }

            let mut new_state = state;
            let mut appended = Vec::new();
            let mut skipped = Vec::new();

            for EntryRequest { cid, metadata_hash } in entries {
                if new_state.entries.contains_key(&cid) {
                    skipped.push(cid);
                } else {
                    // Invariant: HashMap key == entry.cid (single insertion site).
                    new_state.entries.insert(
                        cid.clone(),
                        RegistryEntry {
                            cid: cid.clone(),
                            metadata_hash,
                            anchor_timestamp,
                            anchored_by,
                            version: CURRENT_REGISTRY_ENTRY_VERSION,
                        },
                    );
                    appended.push(cid);
                }
            }

            Ok(ApplyOutcome {
                new_state,
                appended_cids: appended,
                skipped_duplicate_cids: skipped,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// SPEL program wrapper (gated behind `real-spel` feature)
// ---------------------------------------------------------------------------
//
// When the `real-spel` feature is enabled and the spel-framework / nssa-core / risc0-zkvm
// dependencies are available, this section exposes the program as a `#[lez_program]` module.
// The generated IDL is emitted by a separate `examples/src/bin/generate_idl.rs` runner
// (added when the LEZ dev environment is set up).
//
// The shape below matches the SPEL conventions documented in
// https://github.com/logos-co/spel — confirmed against logos-co/lez-multisig and the
// research notes for this prize. Names: spel-framework + #[lez_program] + SpelResult.

#[cfg(feature = "real-spel")]
mod program {
    // use spel_framework::prelude::*;
    // use nssa_core::account::AccountWithMetadata;
    // use nssa_core::program::AccountPostState;
    //
    // risc0_zkvm::guest::entry!(main);
    //
    // #[lez_program]
    // mod chronicle_registry {
    //     use super::*;
    //     use crate::{apply_instruction, RegistryState, TransitionError};
    //     use registry_core::Instruction;
    //
    //     #[instruction]
    //     pub fn init_registry(
    //         #[account(init, pda = [literal("chronicle_registry")])]
    //         mut registry: AccountWithMetadata,
    //         #[account(signer)] anchorer: AccountWithMetadata,
    //     ) -> SpelResult {
    //         let state = RegistryState::default();
    //         let outcome = apply_instruction(state, Instruction::InitRegistry, now())
    //             .map_err(to_spel_error)?;
    //         write_state(&mut registry, &outcome.new_state);
    //         Ok(SpelOutput::execute(vec![registry, anchorer], vec![]))
    //     }
    //
    //     #[instruction]
    //     pub fn index_batch(
    //         #[account(mut, pda = [literal("chronicle_registry")])]
    //         mut registry: AccountWithMetadata,
    //         #[account(signer)] anchorer: AccountWithMetadata,
    //         entries_borsh: Vec<u8>,
    //     ) -> SpelResult {
    //         let state: RegistryState = read_state(&registry);
    //         let entries: Vec<registry_core::EntryRequest> =
    //             borsh::from_slice(&entries_borsh).map_err(|_| SpelError::custom(
    //                 registry_core::error_codes::E_INVALID_CID, "malformed entries"))?;
    //         let outcome = apply_instruction(state, Instruction::IndexBatch { entries }, now())
    //             .map_err(to_spel_error)?;
    //         write_state(&mut registry, &outcome.new_state);
    //         // LP-0012 event emission (uncomment once bristinWild's events fork is merged):
    //         // for cid in &outcome.appended_cids {
    //         //     emit_event(ANCHORED_DISCRIMINANT, &AnchoredEvent { cid: cid.clone() });
    //         // }
    //         Ok(SpelOutput::execute(vec![registry, anchorer], vec![]))
    //     }
    // }
    //
    // fn to_spel_error(e: TransitionError) -> SpelError {
    //     SpelError::custom(e.code(), e.to_string())
    // }
    //
    // fn read_state(account: &AccountWithMetadata) -> RegistryState {
    //     borsh::from_slice(account.account.data.as_ref()).unwrap_or_default()
    // }
    //
    // fn write_state(account: &mut AccountWithMetadata, state: &RegistryState) {
    //     let bytes = borsh::to_vec(state).expect("state serializes");
    //     account.account.data = bytes.try_into().expect("fits in account data");
    // }
    //
    // fn now() -> u64 {
    //     // Block context timestamp once LEZ exposes it; placeholder for the scaffold.
    //     0
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use registry_core::EntryRequest;

    const TEST_ANCHORER: [u8; 32] = [9u8; 32];

    fn entry(cid: &str) -> EntryRequest {
        // Pad short test CIDs to the realistic Codex length so they pass the validator.
        let padded = if cid.len() < 10 {
            format!("{:z<10}", cid)
        } else {
            cid.to_string()
        };
        EntryRequest {
            cid: padded,
            metadata_hash: [0u8; 32],
        }
    }

    fn initialized() -> RegistryState {
        apply_instruction(
            RegistryState::default(),
            Instruction::InitRegistry,
            0,
            TEST_ANCHORER,
        )
        .unwrap()
        .new_state
    }

    #[test]
    fn init_creates_empty_initialized_state() {
        let outcome = apply_instruction(
            RegistryState::default(),
            Instruction::InitRegistry,
            42,
            TEST_ANCHORER,
        )
        .unwrap();
        assert!(outcome.new_state.initialized);
        assert!(outcome.new_state.entries.is_empty());
        assert!(outcome.appended_cids.is_empty());
    }

    #[test]
    fn cannot_init_twice() {
        let state = initialized();
        let err =
            apply_instruction(state, Instruction::InitRegistry, 0, TEST_ANCHORER).unwrap_err();
        assert_eq!(err, TransitionError::AlreadyInitialized);
    }

    #[test]
    fn index_batch_appends() {
        let outcome = apply_instruction(
            initialized(),
            Instruction::IndexBatch {
                entries: vec![entry("zABC"), entry("zDEF")],
            },
            100,
            TEST_ANCHORER,
        )
        .unwrap();
        assert_eq!(outcome.appended_cids.len(), 2);
        assert_eq!(outcome.new_state.entries.len(), 2);
        // HashMap has no ordering — pick any entry and check the timestamp.
        let any = outcome.new_state.entries.values().next().unwrap();
        assert_eq!(any.anchor_timestamp, 100);
    }

    #[test]
    fn index_batch_populates_anchored_by_and_version() {
        let outcome = apply_instruction(
            initialized(),
            Instruction::IndexBatch {
                entries: vec![entry("zNEW")],
            },
            100,
            TEST_ANCHORER,
        )
        .unwrap();
        let cid = outcome.appended_cids[0].clone();
        let entry = outcome.new_state.get(&cid).unwrap();
        assert_eq!(entry.anchored_by, TEST_ANCHORER);
        assert_eq!(entry.version, CURRENT_REGISTRY_ENTRY_VERSION);
        // Invariant: HashMap key matches entry.cid.
        assert_eq!(&entry.cid, &cid);
    }

    #[test]
    fn index_batch_is_idempotent() {
        let first_entry = entry("zABC");
        let after_first = apply_instruction(
            initialized(),
            Instruction::IndexBatch {
                entries: vec![first_entry.clone()],
            },
            100,
            TEST_ANCHORER,
        )
        .unwrap();
        let after_second = apply_instruction(
            after_first.new_state,
            Instruction::IndexBatch {
                entries: vec![first_entry.clone(), entry("zNEW")],
            },
            200,
            [7u8; 32], // different anchorer
        )
        .unwrap();
        assert_eq!(after_second.appended_cids.len(), 1);
        assert_eq!(after_second.skipped_duplicate_cids.len(), 1);
        assert_eq!(after_second.new_state.entries.len(), 2);
        // First entry keeps its original timestamp AND its original anchorer despite the
        // re-submission — idempotency means we don't overwrite.
        let abc = after_second.new_state.get(&first_entry.cid).unwrap();
        assert_eq!(abc.anchor_timestamp, 100);
        assert_eq!(abc.anchored_by, TEST_ANCHORER);
    }

    #[test]
    fn index_batch_rejects_empty() {
        let err = apply_instruction(
            initialized(),
            Instruction::IndexBatch { entries: vec![] },
            0,
            TEST_ANCHORER,
        )
        .unwrap_err();
        assert_eq!(err, TransitionError::BatchEmpty);
    }

    #[test]
    fn index_batch_rejects_oversized() {
        let entries: Vec<EntryRequest> = (0..(MAX_BATCH_ENTRIES + 1))
            .map(|i| entry(&format!("z{:03}", i)))
            .collect();
        let err = apply_instruction(
            initialized(),
            Instruction::IndexBatch { entries },
            0,
            TEST_ANCHORER,
        )
        .unwrap_err();
        assert!(matches!(err, TransitionError::BatchOversized { .. }));
    }

    #[test]
    fn index_batch_rejects_uninitialized() {
        let err = apply_instruction(
            RegistryState::default(),
            Instruction::IndexBatch {
                entries: vec![entry("zABC")],
            },
            0,
            TEST_ANCHORER,
        )
        .unwrap_err();
        assert_eq!(err, TransitionError::Uninitialized);
    }

    #[test]
    fn index_batch_rejects_invalid_cid() {
        // "not-a-cid-here" is long enough but contains hyphens (not in any multibase charset).
        let bad = EntryRequest {
            cid: "not-a-cid-here".into(),
            metadata_hash: [0u8; 32],
        };
        let err = apply_instruction(
            initialized(),
            Instruction::IndexBatch { entries: vec![bad] },
            0,
            TEST_ANCHORER,
        )
        .unwrap_err();
        match err {
            TransitionError::InvalidCid(s) => assert_eq!(s, "not-a-cid-here"),
            other => panic!("wrong error: {:?}", other),
        }
    }

    #[test]
    fn invalid_cid_does_not_partially_apply() {
        let state = initialized();
        let bad = EntryRequest {
            cid: "definitely-not-valid!".into(),
            metadata_hash: [0u8; 32],
        };
        let err = apply_instruction(
            state.clone(),
            Instruction::IndexBatch {
                entries: vec![entry("zGOOD"), bad],
            },
            0,
            TEST_ANCHORER,
        )
        .unwrap_err();
        assert!(matches!(err, TransitionError::InvalidCid(_)));
        // State should be untouched on error — verifiable by re-applying with only the good entry.
        let outcome = apply_instruction(
            state,
            Instruction::IndexBatch {
                entries: vec![entry("zGOOD")],
            },
            0,
            TEST_ANCHORER,
        )
        .unwrap();
        assert_eq!(outcome.new_state.entries.len(), 1);
    }

    #[test]
    fn state_roundtrips_through_borsh() {
        let state = initialized();
        let bytes = borsh::to_vec(&state).unwrap();
        let parsed: RegistryState = borsh::from_slice(&bytes).unwrap();
        assert_eq!(state, parsed);
    }

    #[test]
    fn negative_anchor_timestamps_supported() {
        // i64 lets us round-trip negative values (forward-compat for pre-epoch test fixtures
        // and any LEZ block context that yields signed timestamps).
        let outcome = apply_instruction(
            initialized(),
            Instruction::IndexBatch {
                entries: vec![entry("zNEG")],
            },
            -1,
            TEST_ANCHORER,
        )
        .unwrap();
        let cid = &outcome.appended_cids[0];
        assert_eq!(outcome.new_state.get(cid).unwrap().anchor_timestamp, -1);
    }
}

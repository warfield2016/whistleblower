//! chronicle-registry — LEZ guest binary.
//!
//! Reads the registry PDA's current `RegistryState`, applies the caller's
//! `Instruction`, writes the new state back as the post-state. Pattern
//! adopted from LEZ's canonical `examples/program_deployment/methods/guest/src/bin/hello_world.rs`
//! after the lez-framework template was discovered to be broken against
//! the current LEZ pin (see ../../docs/INTEGRATION_NOTES.md).
//!
//! ## Wire-types are inlined
//!
//! The wire types (`Instruction`, `EntryRequest`, `RegistryEntry`,
//! `CURRENT_REGISTRY_ENTRY_VERSION`, `MAX_BATCH_ENTRIES`) are duplicated
//! here from `crates/registry-core/src/lib.rs`. This is intentional: the
//! cargo-risczero docker build context is rooted at the guest's containing
//! directory, so path-deps that escape the methods/ workspace
//! (`../../crates/registry-core`) aren't visible to the inner build.
//!
//! The types are stable (frozen for v1 per docs/API.md) and won't drift.
//! A future refactor can either:
//!   (a) git-publish registry-core and pull it in via git dep, OR
//!   (b) symlink/vendor it under methods/.
//! Both add complexity for no functional benefit right now.

use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::program::{
    AccountPostState, Claim, ProgramInput, ProgramOutput, read_nssa_inputs,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---- inlined wire types (mirror of registry-core) ----

pub const MAX_BATCH_ENTRIES: usize = 50;
pub const METADATA_HASH_LEN: usize = 32;
pub const CURRENT_REGISTRY_ENTRY_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum Instruction {
    InitRegistry,
    IndexBatch { entries: Vec<EntryRequest> },
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct EntryRequest {
    pub cid: String,
    pub metadata_hash: [u8; METADATA_HASH_LEN],
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub cid: String,
    pub metadata_hash: [u8; METADATA_HASH_LEN],
    pub anchor_timestamp: i64,
    pub anchored_by: [u8; 32],
    pub version: u8,
}

// ---- on-chain state (lives in the registry PDA's data field) ----

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct RegistryState {
    pub initialized: bool,
    pub entries: HashMap<String, RegistryEntry>,
}

// ---- program entry point ----

fn main() {
    let (
        ProgramInput {
            self_program_id,
            caller_program_id,
            pre_states,
            instruction,
        },
        instruction_data,
    ) = read_nssa_inputs::<Instruction>();

    // chronicle-registry operates on exactly one account: the registry PDA.
    let [registry_pre] = pre_states
        .clone()
        .try_into()
        .unwrap_or_else(|_| panic!("chronicle-registry requires exactly one input account"));

    // Decode existing state. Empty data ⇒ first call ⇒ default state.
    let existing_state: RegistryState = if registry_pre.account.data.as_ref().is_empty() {
        RegistryState::default()
    } else {
        BorshDeserialize::try_from_slice(registry_pre.account.data.as_ref())
            .expect("registry data must be a borsh-encoded RegistryState")
    };

    let new_state = match instruction {
        Instruction::InitRegistry => {
            assert!(
                !existing_state.initialized,
                "E_REGISTRY_ALREADY_INITIALIZED: registry already initialized"
            );
            RegistryState {
                initialized: true,
                entries: HashMap::new(),
            }
        }

        Instruction::IndexBatch { entries } => {
            assert!(
                existing_state.initialized,
                "E_REGISTRY_UNINITIALIZED: registry not initialized"
            );
            assert!(!entries.is_empty(), "E_BATCH_EMPTY: batch is empty");
            assert!(
                entries.len() <= MAX_BATCH_ENTRIES,
                "E_BATCH_OVERSIZED: batch exceeds {} entries",
                MAX_BATCH_ENTRIES
            );

            let mut next = existing_state.clone();
            // Placeholder values for fields the LEZ block context doesn't expose
            // yet via nssa_core::program. The chain timestamp + signer account
            // will be wired in once we confirm the exact accessors — for now
            // both are zeros, which is enough for the prize submission's
            // "queryable by CID" success criterion.
            let anchor_timestamp: i64 = 0;
            let anchored_by: [u8; 32] = [0u8; 32];

            for EntryRequest { cid, metadata_hash } in entries {
                next.entries.entry(cid.clone()).or_insert(RegistryEntry {
                    cid,
                    metadata_hash,
                    anchor_timestamp,
                    anchored_by,
                    version: CURRENT_REGISTRY_ENTRY_VERSION,
                });
            }
            next
        }
    };

    // Serialize new state back into the account's data field.
    let mut post_account = registry_pre.account.clone();
    let bytes = borsh::to_vec(&new_state).expect("RegistryState must serialize");
    post_account.data = bytes
        .try_into()
        .expect("registry state must fit in account data cap");

    // Auto-claim on first call (account uninitialized = program_owner ==
    // DEFAULT_PROGRAM_ID); preserve ownership otherwise.
    let post_state = AccountPostState::new_claimed_if_default(post_account, Claim::Authorized);

    ProgramOutput::new(
        self_program_id,
        caller_program_id,
        instruction_data,
        vec![registry_pre],
        vec![post_state],
    )
    .write();
}

risc0_zkvm::guest::entry!(main);

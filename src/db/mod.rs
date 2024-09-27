// TODO: Fix me
#![allow(dead_code)]

use jsonrpsee::core::Serialize;
use serde::Deserialize;

mod data_availability_db;
pub(crate) mod lfs;

/// TODO: we need a trait to abstract the database operations in order to support multiple databases

pub trait Database: Send + Sync {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn put(&self, key: Vec<u8>, value: Vec<u8>);
    fn del(&self, key: Vec<u8>) -> Option<Vec<u8>>;
}

/// Used to represent different tables or columns or databases
/// Which is more appropriate to use among tables, columns, and databases to represent our data?
pub(crate) mod columns {
    /// The number of columns in the DB
    pub const TOTAL_COLUMNS: usize = 2;

    /// The column for DEFAULT
    pub const DEFAULT: usize = 0;

    // TODO: others
    /// The column for DATA_AVAILABILITY
    pub const DATA_AVAILABILITY: usize = 1;
}

pub(crate) mod keys {
    pub const KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER: &[u8] =
        b"LAST_SEQUENCE_FINALITY_BLOCK_NUMBER";
    pub const KEY_NEXT_BATCH: &[u8] = b"NEXT_BATCH";
    pub const KEY_LAST_SUBMITTED_BLOCK_NUMBER: &[u8] = b"LAST_SUBMITTED_BLOCK_NUMBER";
    pub const KEY_LAST_PROVEN_BLOCK_NUMBER: &[u8] = b"LAST_PROVEN_BLOCK_NUMBER";
    pub const KEY_LAST_VERIFIED_BLOCK_NUMBER: &[u8] = b"LAST_VERIFIED_BLOCK_NUMBER";
    pub const KEY_PROVE_STEP_RECORD: &[u8] = b"PROVE_STEP_RECORD";
    pub const KEY_LAST_VERIFIED_BATCH_NUMBER: &[u8] = b"LAST_VERIFIED_BATCH_NUMBER";
}

pub(crate) mod prefix {
    pub const PREFIX_BATCH_PROOF: &[u8] = b"BATCH_PROOF_";
    pub const PREFIX_BLOCK_STATUS: &[u8] = b"BLOCK_STATUS_";
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// the tx is pending
    Pending,
    /// confirmed by sequencer
    Sequenced,
    /// packing the block into a batch
    Batching,
    /// confirmed by DA
    /// TODO: we skip the DA for now, should support it in the future
    Submitted,
    /// confirmed by settlement
    Finalized,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofResult {
    // TODO: refactor to batch
    pub block_number: u64,
    pub proof: String,
    pub public_input: String,
    pub pre_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
}

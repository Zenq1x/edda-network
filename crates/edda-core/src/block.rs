use serde::{Deserialize, Serialize};

use crate::account::Pubkey;
use crate::hash::{hashv, Hash};
use crate::transaction::Transaction;

/// Number of slots per epoch (determines staking/voting windows)
pub const SLOTS_PER_EPOCH: u64 = 432_000;

/// Target time per slot in milliseconds
pub const SLOT_MS: u64 = 400;

/// Base fee burned per transaction (in lamports) — EIP-1559 style
pub const BASE_FEE_LAMPORTS: u64 = 5_000;

/// Maximum validator tip per transaction (in lamports)
pub const MAX_TIP_LAMPORTS: u64 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Slot number — monotonically increasing, like a block number
    pub slot: u64,
    pub parent_slot: u64,
    /// Hash of this block's full content
    pub blockhash: Hash,
    pub parent_blockhash: Hash,
    /// PoH hash at the moment this block was sealed
    pub poh_hash: Hash,
    /// Unix timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Validator (leader) that produced this block
    pub leader: Pubkey,
    pub transaction_count: u32,
    /// Total fees collected this slot (lamports)
    pub total_fees: u64,
    /// Portion of fees burned (lamports)
    pub fees_burned: u64,
    /// Portion paid to the leader validator (lamports)
    pub fees_to_validator: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Recompute the blockhash from slot + parent + poh hash
    pub fn compute_blockhash(&self) -> Hash {
        hashv(&[
            &self.header.slot.to_le_bytes(),
            self.header.parent_blockhash.as_bytes(),
            self.header.poh_hash.as_bytes(),
        ])
    }

    pub fn verify_blockhash(&self) -> bool {
        self.compute_blockhash() == self.header.blockhash
    }
}

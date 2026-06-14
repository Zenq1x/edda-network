use edda_core::{account::Pubkey, hash::Hash};
use serde::{Deserialize, Serialize};

/// A validator's vote on a specific slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// The slot being voted on
    pub slot: u64,
    /// Hash of the block at this slot (prevents equivocation)
    pub hash: Hash,
    /// Timestamp of the vote (ms)
    pub timestamp: u64,
}

/// A signed vote transaction broadcast to the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteTransaction {
    pub validator: Pubkey,
    pub vote:      Vote,
    /// Ed25519 signature over the serialized Vote
    pub signature: Vec<u8>,
}

impl VoteTransaction {
    pub fn new(validator: Pubkey, slot: u64, hash: Hash, timestamp: u64) -> Self {
        Self {
            validator,
            vote: Vote { slot, hash, timestamp },
            signature: Vec::new(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self.vote).expect("vote serialization is infallible")
    }
}

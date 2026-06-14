use serde::{Deserialize, Serialize};

use crate::account::Pubkey;
use crate::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        Self { pubkey, is_signer, is_writable: true }
    }

    pub fn readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self { pubkey, is_signer, is_writable: false }
    }
}

/// A single program call inside a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    /// The WASM program to invoke (system program = native transfers)
    pub program_id: Pubkey,
    /// Accounts this instruction reads or writes
    pub accounts: Vec<AccountMeta>,
    /// Raw bytes passed to the program as input
    pub data: Vec<u8>,
}

/// The part of a transaction that gets signed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Account that pays the transaction fee
    pub fee_payer: Pubkey,
    /// Prevents replay — must match a recent block hash
    pub recent_blockhash: Hash,
    pub instructions: Vec<Instruction>,
}

impl Message {
    pub fn new(
        fee_payer: Pubkey,
        recent_blockhash: Hash,
        instructions: Vec<Instruction>,
    ) -> Self {
        Self { fee_payer, recent_blockhash, instructions }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("message serialization is infallible")
    }
}

/// A fully signed transaction ready to be submitted to the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub message: Message,
    /// Ed25519 signature over the serialized message (64 bytes)
    pub signature: Vec<u8>,
    pub signer: Pubkey,
}

impl Transaction {
    /// Hash of this transaction — used to mix into PoH
    pub fn hash(&self) -> Hash {
        Hash::new(&self.message.to_bytes())
    }
}

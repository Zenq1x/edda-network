use serde::{Deserialize, Serialize};
use std::fmt;

pub const PUBKEY_BYTES: usize = 32;

/// 1 EDDA = 1_000_000_000 lamports (same ratio as SOL for familiarity)
pub const LAMPORTS_PER_EDDA: u64 = 1_000_000_000;

/// Max supply: 500 million EDDA (like a fixed Bitcoin-style cap)
pub const MAX_SUPPLY_EDDA: u64 = 500_000_000;
pub const MAX_SUPPLY_LAMPORTS: u64 = MAX_SUPPLY_EDDA * LAMPORTS_PER_EDDA;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pubkey(pub [u8; PUBKEY_BYTES]);

impl Pubkey {
    pub fn new(bytes: [u8; PUBKEY_BYTES]) -> Self {
        Pubkey(bytes)
    }

    /// The system program owns all basic (non-contract) accounts
    pub fn system_program() -> Self {
        Pubkey([0u8; PUBKEY_BYTES])
    }

    /// The burn address — fees sent here are gone forever
    pub fn burn_address() -> Self {
        Pubkey([0xFF; PUBKEY_BYTES])
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pubkey({}...)", hex::encode(&self.0[..6]))
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// An account in the Edda Network state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Balance in lamports (1 EDDA = 1_000_000_000 lamports)
    pub lamports: u64,
    /// Raw data stored in this account (WASM bytecode for programs)
    pub data: Vec<u8>,
    /// Which program owns and can write to this account
    pub owner: Pubkey,
    /// If true, this account holds a deployable WASM smart contract
    pub executable: bool,
}

impl Account {
    pub fn new(lamports: u64, owner: Pubkey) -> Self {
        Self {
            lamports,
            data: Vec::new(),
            owner,
            executable: false,
        }
    }

    pub fn new_program(bytecode: Vec<u8>, owner: Pubkey) -> Self {
        Self {
            lamports: 0,
            data: bytecode,
            owner,
            executable: true,
        }
    }

    pub fn edda_balance(&self) -> f64 {
        self.lamports as f64 / LAMPORTS_PER_EDDA as f64
    }
}

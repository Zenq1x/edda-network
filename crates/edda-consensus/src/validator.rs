use std::collections::HashMap;

use edda_core::account::Pubkey;

/// One validator in the Edda Network
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    /// The validator's signing key (identity)
    pub identity:   Pubkey,
    /// The account where delegators stake EDDA
    pub vote_account: Pubkey,
    /// Amount of EDDA staked (in lamports)
    pub stake:      u64,
    /// Percent of block rewards kept by validator (0–100)
    pub commission: u8,
    /// Is this validator currently active?
    pub active:     bool,
}

impl ValidatorInfo {
    pub fn new(identity: Pubkey, vote_account: Pubkey, stake: u64, commission: u8) -> Self {
        Self { identity, vote_account, stake, commission, active: true }
    }
}

/// The global registry of all staked validators
pub struct StakePool {
    validators:  HashMap<Pubkey, ValidatorInfo>,
    total_stake: u64,
}

impl StakePool {
    pub fn new() -> Self {
        Self { validators: HashMap::new(), total_stake: 0 }
    }

    pub fn register(&mut self, info: ValidatorInfo) {
        self.total_stake += info.stake;
        self.validators.insert(info.identity, info);
    }

    pub fn get(&self, identity: &Pubkey) -> Option<&ValidatorInfo> {
        self.validators.get(identity)
    }

    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }

    /// Minimum stake needed for a supermajority (2/3 + 1)
    pub fn supermajority_stake(&self) -> u64 {
        self.total_stake * 2 / 3 + 1
    }

    /// Count how much stake has voted for `slot`
    pub fn stake_voted_for(&self, votes: &HashMap<Pubkey, u64>, slot: u64) -> u64 {
        votes
            .iter()
            .filter(|(_, &voted_slot)| voted_slot >= slot)
            .filter_map(|(pk, _)| self.validators.get(pk))
            .map(|v| v.stake)
            .sum()
    }

    /// True if `slot` has been voted on by 2/3+ of total stake
    pub fn is_confirmed(&self, votes: &HashMap<Pubkey, u64>, slot: u64) -> bool {
        self.stake_voted_for(votes, slot) >= self.supermajority_stake()
    }

    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ValidatorInfo> {
        self.validators.values()
    }
}

impl Default for StakePool {
    fn default() -> Self {
        Self::new()
    }
}

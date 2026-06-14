use edda_core::hash::Hash;
use sha2::{Digest, Sha256};

/// One entry in the PoH sequence.
///
/// Each entry proves that a certain number of SHA-256 hash steps were
/// performed since the previous entry. If `mixin` is set, a transaction
/// hash was woven into the chain at this point, timestamping it permanently.
#[derive(Debug, Clone)]
pub struct PohEntry {
    /// How many hashes were computed to reach this entry
    pub num_hashes: u64,
    /// The hash value at this point in the chain
    pub hash: Hash,
    /// Transaction batch hash mixed in at this point (if any)
    pub mixin: Option<Hash>,
}

/// The PoH recorder — the heart of the Edda Network's clock.
///
/// This runs continuously on the leader validator, hashing as fast as
/// the CPU allows. The speed of hashing is what proves time has passed —
/// no one can fake a long sequence without actually spending the compute.
pub struct PohRecorder {
    pub current_hash: Hash,
    pub num_hashes: u64,
    pub tick_height: u64,
    hashes_since_last_entry: u64,
}

impl PohRecorder {
    pub fn new(genesis_hash: Hash) -> Self {
        Self {
            current_hash: genesis_hash,
            num_hashes: 0,
            tick_height: 0,
            hashes_since_last_entry: 0,
        }
    }

    fn step(&mut self) {
        let mut h = Sha256::new();
        h.update(self.current_hash.as_bytes());
        self.current_hash = Hash::new_from_array(h.finalize().into());
        self.num_hashes += 1;
        self.hashes_since_last_entry += 1;
    }

    /// Run N pure hash steps (simulates compute time passing)
    pub fn hash_n(&mut self, n: u64) {
        for _ in 0..n {
            self.step();
        }
    }

    /// Produce a tick — marks the passage of one time unit
    pub fn tick(&mut self) -> PohEntry {
        self.step();
        let entry = PohEntry {
            num_hashes: self.hashes_since_last_entry,
            hash: self.current_hash,
            mixin: None,
        };
        self.tick_height += 1;
        self.hashes_since_last_entry = 0;
        entry
    }

    /// Record a transaction by mixing its hash into the PoH chain.
    ///
    /// This permanently timestamps the transaction — anyone can verify
    /// it existed at this exact point in the chain's history.
    pub fn record(&mut self, mixin: Hash) -> PohEntry {
        let mut h = Sha256::new();
        h.update(self.current_hash.as_bytes());
        h.update(mixin.as_bytes());
        self.current_hash = Hash::new_from_array(h.finalize().into());
        self.num_hashes += 1;
        self.hashes_since_last_entry += 1;

        let entry = PohEntry {
            num_hashes: self.hashes_since_last_entry,
            hash: self.current_hash,
            mixin: Some(mixin),
        };
        self.hashes_since_last_entry = 0;
        entry
    }
}

/// Verify a PoH sequence independently.
///
/// Any node can replay the hash chain from `start_hash` and confirm
/// every entry is correct — this is what makes PoH trustless.
pub fn verify_entries(start_hash: Hash, entries: &[PohEntry]) -> bool {
    let mut current = start_hash;

    for entry in entries {
        if entry.num_hashes == 0 {
            return false;
        }
        // Replay n-1 pure hash steps
        for _ in 0..(entry.num_hashes - 1) {
            let mut h = Sha256::new();
            h.update(current.as_bytes());
            current = Hash::new_from_array(h.finalize().into());
        }
        // Final step — include mixin if present
        let mut h = Sha256::new();
        h.update(current.as_bytes());
        if let Some(mixin) = &entry.mixin {
            h.update(mixin.as_bytes());
        }
        current = Hash::new_from_array(h.finalize().into());

        if current != entry.hash {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poh_verify_ticks() {
        let genesis = Hash::new(b"edda-genesis");
        let mut poh = PohRecorder::new(genesis);
        let entries: Vec<PohEntry> = (0..10).map(|_| poh.tick()).collect();
        assert!(verify_entries(genesis, &entries));
    }

    #[test]
    fn poh_verify_with_mixin() {
        let genesis = Hash::new(b"edda-genesis");
        let mut poh = PohRecorder::new(genesis);
        let mut entries = Vec::new();
        entries.push(poh.tick());
        entries.push(poh.tick());
        entries.push(poh.record(Hash::new(b"transaction-abc")));
        entries.push(poh.tick());
        assert!(verify_entries(genesis, &entries));
    }

    #[test]
    fn poh_tampered_entry_fails() {
        let genesis = Hash::new(b"edda-genesis");
        let mut poh = PohRecorder::new(genesis);
        let mut entries: Vec<PohEntry> = (0..5).map(|_| poh.tick()).collect();
        // Tamper with one entry
        entries[2].hash = Hash::new(b"fake");
        assert!(!verify_entries(genesis, &entries));
    }
}

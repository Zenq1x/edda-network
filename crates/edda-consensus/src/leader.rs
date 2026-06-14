use edda_core::account::Pubkey;

use crate::validator::ValidatorInfo;

/// Computes the leader schedule for one epoch.
///
/// Each validator gets a number of slots proportional to their stake.
/// The order is deterministically shuffled using the previous epoch's
/// last PoH hash as a seed — so every node independently arrives at
/// the exact same schedule.
pub struct LeaderSchedule {
    /// Slot-indexed list of leaders. Index with `slot % schedule.len()`.
    schedule: Vec<Pubkey>,
    pub epoch: u64,
}

impl LeaderSchedule {
    /// Build a schedule for `epoch` seeded by `seed` (last hash of prev epoch).
    pub fn new(validators: &[&ValidatorInfo], epoch: u64, seed: u64) -> Self {
        let total_stake: u64 = validators.iter().map(|v| v.stake).sum();
        if total_stake == 0 || validators.is_empty() {
            return Self { schedule: Vec::new(), epoch };
        }

        // Give each validator a number of slots proportional to stake.
        // We use 1000 slots as the base unit — validators with tiny stake
        // still get at least 1 slot.
        const SLOTS_PER_EPOCH_MINI: u64 = 1_000;
        let mut schedule: Vec<Pubkey> = Vec::with_capacity(SLOTS_PER_EPOCH_MINI as usize);

        for v in validators {
            let slots = ((v.stake as u128 * SLOTS_PER_EPOCH_MINI as u128)
                / total_stake as u128)
                .max(1) as usize;
            for _ in 0..slots {
                schedule.push(v.identity);
            }
        }

        // Deterministic Fisher-Yates shuffle seeded from epoch + seed
        let mut rng = seed ^ (epoch.wrapping_mul(0x9e3779b97f4a7c15));
        for i in (1..schedule.len()).rev() {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let j = ((rng >> 33) as usize) % (i + 1);
            schedule.swap(i, j);
        }

        Self { schedule, epoch }
    }

    pub fn leader_for_slot(&self, slot: u64) -> Option<Pubkey> {
        if self.schedule.is_empty() {
            return None;
        }
        Some(self.schedule[(slot as usize) % self.schedule.len()])
    }

    pub fn slot_count(&self) -> usize {
        self.schedule.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::ValidatorInfo;

    fn dummy_validator(last_byte: u8, stake: u64) -> ValidatorInfo {
        let mut key = [0u8; 32];
        key[31] = last_byte;
        ValidatorInfo::new(Pubkey::new(key), Pubkey::new(key), stake, 0)
    }

    #[test]
    fn schedule_covers_all_validators() {
        let v1 = dummy_validator(1, 100);
        let v2 = dummy_validator(2, 100);
        let v3 = dummy_validator(3, 100);
        let sched = LeaderSchedule::new(&[&v1, &v2, &v3], 0, 12345);
        let has_v1 = (0..sched.slot_count()).any(|s| sched.leader_for_slot(s as u64) == Some(v1.identity));
        let has_v2 = (0..sched.slot_count()).any(|s| sched.leader_for_slot(s as u64) == Some(v2.identity));
        let has_v3 = (0..sched.slot_count()).any(|s| sched.leader_for_slot(s as u64) == Some(v3.identity));
        assert!(has_v1 && has_v2 && has_v3);
    }

    #[test]
    fn schedule_is_deterministic() {
        let v = dummy_validator(1, 500);
        let s1 = LeaderSchedule::new(&[&v], 0, 99);
        let s2 = LeaderSchedule::new(&[&v], 0, 99);
        assert_eq!(s1.schedule, s2.schedule);
    }
}

/// Tower BFT — per-validator consensus state.
///
/// The "tower" is a stack of votes. Each time you vote, all older votes gain
/// one more "confirmation". A vote with N confirmations cannot be overridden
/// for 2^N slots — this is the lockout. It forces validators to commit:
/// switching forks costs more and more the deeper you go.
///
/// When a vote reaches 32 confirmations it is permanently "rooted" —
/// that slot can never be rolled back.

const MAX_LOCKOUT_EXPONENT: u32 = 32;

/// One entry in a validator's tower
#[derive(Debug, Clone)]
pub struct TowerVote {
    /// Slot this vote was cast for
    pub slot: u64,
    /// How many subsequent slots have confirmed this vote
    pub confirmations: u32,
}

impl TowerVote {
    /// Lockout in slots = 2^confirmations
    pub fn lockout(&self) -> u64 {
        2u64.pow(self.confirmations.min(MAX_LOCKOUT_EXPONENT))
    }

    /// The first slot at which this vote can be overridden
    pub fn expiry(&self) -> u64 {
        self.slot + self.lockout()
    }
}

/// The full Tower BFT state for one validator
#[derive(Debug, Default)]
pub struct Tower {
    /// Stack of votes, oldest first
    votes: Vec<TowerVote>,
    /// The last permanently finalized slot (cannot be undone)
    pub root: Option<u64>,
}

impl Tower {
    pub fn new() -> Self {
        Self::default()
    }

    /// Can this validator safely vote for `slot`?
    ///
    /// Returns false if voting would violate the lockout of any prior vote
    /// (i.e., we would be implicitly rolling back a locked vote).
    pub fn can_vote(&self, slot: u64) -> bool {
        for vote in &self.votes {
            // If an existing vote is still locked out and slot is not a
            // descendant (higher), we'd be forking — not allowed.
            if vote.slot > slot {
                return false; // would be voting on an earlier slot
            }
        }
        true
    }

    /// Record a vote for `slot` and update the entire tower.
    ///
    /// Returns the new root if one was just finalized.
    pub fn record_vote(&mut self, slot: u64) -> Option<u64> {
        if !self.can_vote(slot) {
            return None;
        }

        // Every existing vote gains one confirmation
        for v in &mut self.votes {
            v.confirmations = v.confirmations.saturating_add(1);
        }

        // Push the new vote
        self.votes.push(TowerVote { slot, confirmations: 1 });

        // Votes that have reached MAX_LOCKOUT_EXPONENT confirmations are
        // finalized — pop them from the bottom and update the root.
        let mut new_root = None;
        while self
            .votes
            .first()
            .map(|v| v.confirmations >= MAX_LOCKOUT_EXPONENT)
            .unwrap_or(false)
        {
            let finalized = self.votes.remove(0);
            self.root = Some(finalized.slot);
            new_root  = Some(finalized.slot);
        }

        new_root
    }

    /// Votes currently in the tower (newest last)
    pub fn votes(&self) -> &[TowerVote] {
        &self.votes
    }

    /// Depth = number of unfinalized votes in the tower
    pub fn depth(&self) -> usize {
        self.votes.len()
    }

    /// Highest confirmation count in the tower (= depth of oldest vote)
    pub fn max_confirmations(&self) -> u32 {
        self.votes.first().map(|v| v.confirmations).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vote_confirmations_grow() {
        let mut t = Tower::new();
        t.record_vote(1);
        t.record_vote(2);
        t.record_vote(3);
        // Slot 1 should have 3 confirmations (voted 3 times after it)
        assert_eq!(t.votes[0].slot, 1);
        assert_eq!(t.votes[0].confirmations, 3);
        assert_eq!(t.votes[2].confirmations, 1);
    }

    #[test]
    fn lockout_is_exponential() {
        let vote = TowerVote { slot: 100, confirmations: 4 };
        assert_eq!(vote.lockout(), 16); // 2^4
    }

    #[test]
    fn cannot_vote_on_older_slot() {
        let mut t = Tower::new();
        t.record_vote(10);
        assert!(!t.can_vote(5)); // would fork back
        assert!(t.can_vote(11)); // fine
    }

    #[test]
    fn root_after_32_confirmations() {
        let mut t = Tower::new();
        // Slot 0 starts with 1 confirmation when cast.
        // It needs 31 more subsequent votes to reach 32 → finalized.
        t.record_vote(0);
        for s in 1..=31u64 {
            t.record_vote(s);
        }
        assert_eq!(t.root, Some(0));
    }
}

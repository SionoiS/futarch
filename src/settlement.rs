//! Append-only, hash-chained settlement log.

use crate::amount::Amount;
use crate::crypto::hash_tagged;
use crate::encode::TAG_SETTLEMENT;
use crate::error::Error;
use crate::restriction::Restriction;
use crate::types::{Action, Hash, ProposalId, Target, UserId, encode_action, encode_target};

/// Yes cleared threshold + floor; otherwise No is the default winner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutcomeKind {
    NoDefault = 0,
    YesExecuted = 1,
}

/// `(user, amount)` pair used for pot shares and forfeited stakes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payout {
    pub user: UserId,
    pub amount: Amount,
}

/// Durable economic record of one settled proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementEntry {
    pub prev_hash: Hash,
    pub proposal_id: ProposalId,
    pub target: Target,
    pub action: Action,
    pub outcome: OutcomeKind,
    /// Winning-side successful openers and their share of the pot (not stake return).
    pub payouts: Vec<Payout>,
    /// Stakes deducted from losers and forfeiters.
    pub forfeitures: Vec<Payout>,
    /// Users whose balance is set to 1 token if still zero after money flows.
    pub free_mints: Vec<UserId>,
    pub burned: Amount,
    /// Present only when Yes executed a restriction that actually became effective.
    pub executed_restriction: Option<Restriction>,
    pub this_hash: Hash,
}

impl SettlementEntry {
    /// Build an entry, sort lists canonically, and fill `this_hash`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        prev_hash: Hash,
        proposal_id: ProposalId,
        target: Target,
        action: Action,
        outcome: OutcomeKind,
        mut payouts: Vec<Payout>,
        mut forfeitures: Vec<Payout>,
        mut free_mints: Vec<UserId>,
        burned: Amount,
        executed_restriction: Option<Restriction>,
    ) -> Self {
        payouts.sort_by_key(|p| *p.user.as_bytes());
        forfeitures.sort_by_key(|p| *p.user.as_bytes());
        free_mints.sort_by_key(|u| *u.as_bytes());
        let mut entry = Self {
            prev_hash,
            proposal_id,
            target,
            action,
            outcome,
            payouts,
            forfeitures,
            free_mints,
            burned,
            executed_restriction,
            this_hash: Hash::from_bytes([0; 32]),
        };
        entry.this_hash = entry.compute_hash();
        entry
    }

    pub fn compute_hash(&self) -> Hash {
        hash_tagged(TAG_SETTLEMENT, |e| {
            e.bytes32(self.prev_hash.as_bytes());
            e.bytes32(self.proposal_id.as_bytes());
            encode_target(e, self.target);
            encode_action(e, self.action);
            e.u8(self.outcome as u8);
            e.u32(self.payouts.len() as u32);
            for p in &self.payouts {
                e.bytes32(p.user.as_bytes());
                e.u64(p.amount.subunits());
            }
            e.u32(self.forfeitures.len() as u32);
            for p in &self.forfeitures {
                e.bytes32(p.user.as_bytes());
                e.u64(p.amount.subunits());
            }
            e.u32(self.free_mints.len() as u32);
            for u in &self.free_mints {
                e.bytes32(u.as_bytes());
            }
            e.u64(self.burned.subunits());
            match self.executed_restriction {
                None => {
                    e.u8(0);
                }
                Some(r) => {
                    e.u8(1);
                    e.bytes32(r.user.as_bytes());
                    e.u64(r.duration.secs());
                    e.u64(r.start.millis());
                }
            }
        })
    }

    pub fn verify_hash(&self) -> Result<(), Error> {
        if self.this_hash == self.compute_hash() {
            Ok(())
        } else {
            Err(Error::SettlementHashMismatch)
        }
    }
}

/// Check that `entries` form a chain from `genesis_hash`.
pub fn verify_chain(genesis_hash: Hash, entries: &[SettlementEntry]) -> Result<(), Error> {
    let mut prev = genesis_hash;
    for entry in entries {
        entry.verify_hash()?;
        if entry.prev_hash != prev {
            return Err(Error::BrokenHashChain);
        }
        prev = entry.this_hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DurationSecs, MessageId, Timestamp};

    fn dummy() -> SettlementEntry {
        SettlementEntry::new(
            Hash::from_byte(1),
            ProposalId::from_byte(2),
            Target::Message(MessageId::from_byte(3)),
            Action::RemoveMessage,
            OutcomeKind::NoDefault,
            vec![Payout {
                user: UserId::from_byte(9),
                amount: Amount::from_subunits(3),
            }],
            vec![Payout {
                user: UserId::from_byte(8),
                amount: Amount::from_subunits(5),
            }],
            vec![UserId::from_byte(4)],
            Amount::from_subunits(1),
            Some(Restriction {
                user: UserId::from_byte(7),
                duration: DurationSecs::from_secs(60),
                start: Timestamp::from_millis(10),
            }),
        )
    }

    #[test]
    fn hash_covers_body() {
        let a = dummy();
        assert_eq!(a.this_hash, a.compute_hash());
        let mut b = a.clone();
        b.burned = Amount::from_subunits(2);
        assert_ne!(a.compute_hash(), b.compute_hash());
    }

    #[test]
    fn chain_detects_tamper() {
        let e1 = dummy();
        let e2 = SettlementEntry::new(
            e1.this_hash,
            ProposalId::from_byte(5),
            Target::Message(MessageId::from_byte(3)),
            Action::RemoveMessage,
            OutcomeKind::YesExecuted,
            vec![],
            vec![],
            vec![],
            Amount::ZERO,
            None,
        );
        assert!(verify_chain(e1.prev_hash, &[e1.clone(), e2.clone()]).is_ok());
        let mut broken = e2.clone();
        broken.prev_hash = Hash::from_byte(99);
        broken.this_hash = broken.compute_hash();
        assert_eq!(
            verify_chain(e1.prev_hash, &[e1, broken]).unwrap_err(),
            Error::BrokenHashChain
        );
    }
}

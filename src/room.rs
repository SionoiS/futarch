//! Deterministic room state machine.

use std::collections::BTreeMap;

use crate::amount::Amount;
use crate::balances::{Balances, replay};
use crate::command::{Command, Outcome};
use crate::commitment::{FreeCommitment, Opening, PaidCommitment, RevealCommit};
use crate::error::Error;
use crate::genesis::Genesis;
use crate::params::RoomParams;
use crate::payout::split_pot;
use crate::proposal::{Proposal, ProposalView};
use crate::restriction::{Restriction, effective_restriction, supersedes};
use crate::settlement::{OutcomeKind, Payout, SettlementEntry, verify_chain};
use crate::threshold::{decide, weight};
use crate::types::{Action, Direction, Phase, ProposalId, Target, Timestamp, UserId};

/// Live room: genesis + settlement log + ephemeral open proposals.
///
/// Balances are a cache of replaying the log. They are not primary state.
#[derive(Clone, Debug)]
pub struct Room {
    genesis: Genesis,
    settlements: Vec<SettlementEntry>,
    balances: Balances,
    proposals: BTreeMap<ProposalId, Proposal>,
    proposer_index: BTreeMap<UserId, ProposalId>,
    target_action_index: BTreeMap<(Target, Action), ProposalId>,
}

impl Room {
    /// Create a room at genesis. Each founder holds one token; no proposals.
    pub fn genesis(genesis: Genesis) -> Result<Self, Error> {
        let balances = Balances::from_genesis(&genesis);
        Ok(Self {
            genesis,
            settlements: Vec::new(),
            balances,
            proposals: BTreeMap::new(),
            proposer_index: BTreeMap::new(),
            target_action_index: BTreeMap::new(),
        })
    }

    /// Rebuild settled state from the log. In-flight proposals are not restored;
    /// the host must persist those separately if it needs crash recovery.
    pub fn from_log(
        genesis: Genesis,
        entries: impl IntoIterator<Item = SettlementEntry>,
    ) -> Result<Self, Error> {
        let settlements: Vec<SettlementEntry> = entries.into_iter().collect();
        verify_chain(genesis.hash(), &settlements)?;
        let balances = replay(&genesis, &settlements)?;
        Ok(Self {
            genesis,
            settlements,
            balances,
            proposals: BTreeMap::new(),
            proposer_index: BTreeMap::new(),
            target_action_index: BTreeMap::new(),
        })
    }

    pub fn params(&self) -> &RoomParams {
        self.genesis.params()
    }

    pub fn balance(&self, user: &UserId) -> Amount {
        self.balances.get(user)
    }

    pub fn supply(&self) -> Amount {
        self.balances.supply()
    }

    pub fn settlements(&self) -> &[SettlementEntry] {
        &self.settlements
    }

    pub fn open_proposals(&self) -> impl Iterator<Item = ProposalView> + '_ {
        self.proposals.values().map(Proposal::view)
    }

    pub fn proposal(&self, id: &ProposalId) -> Option<ProposalView> {
        self.proposals.get(id).map(Proposal::view)
    }

    pub fn can_open(&self, user: &UserId) -> bool {
        !self.balances.get(user).is_zero() && !self.proposer_index.contains_key(user)
    }

    pub fn can_free_commit(&self, user: &UserId) -> bool {
        self.balances.get(user).is_zero() && !self.has_outstanding_free(user)
    }

    pub fn effective_restriction(&self, user: &UserId, now: Timestamp) -> Option<Restriction> {
        effective_restriction(&self.settlements, user, now)
    }

    /// Available = replayed balance minus locks across all open proposals.
    pub fn available(&self, user: &UserId) -> Amount {
        let locked = self
            .proposals
            .values()
            .fold(Amount::ZERO, |acc, p| acc.saturating_add(p.locked_by(user)));
        self.balances.get(user).saturating_sub(locked)
    }

    /// Advance windows. Does not auto-settle.
    pub fn tick(&mut self, now: Timestamp) -> Vec<Outcome> {
        let mut out = Vec::new();
        for proposal in self.proposals.values_mut() {
            let before = proposal.phase;
            proposal.advance(now);
            if proposal.phase != before {
                out.push(Outcome::PhaseChanged {
                    proposal: proposal.id,
                    phase: proposal.phase,
                });
            }
        }
        out
    }

    pub fn apply(&mut self, cmd: Command, now: Timestamp) -> Result<Outcome, Error> {
        self.tick(now);
        match cmd {
            Command::OpenProposal {
                proposer,
                target,
                action,
                commit,
            } => self.open_proposal(proposer, target, action, commit, now),
            Command::CommitPaid {
                user,
                proposal,
                commit,
            } => self.commit_paid(user, proposal, commit),
            Command::CommitFree {
                user,
                proposal,
                commit,
            } => self.commit_free(user, proposal, commit),
            Command::CommitToReveal {
                user,
                proposal,
                commit,
            } => self.commit_to_reveal(user, proposal, commit),
            Command::Open {
                user,
                proposal,
                opening,
            } => self.open(user, proposal, opening),
            Command::Settle { proposal } => self.settle(proposal, now),
        }
    }

    fn has_outstanding_free(&self, user: &UserId) -> bool {
        self.proposals.values().any(|p| {
            p.commitments
                .get(user)
                .is_some_and(crate::proposal::UserCommitment::is_free)
        })
    }

    fn open_proposal(
        &mut self,
        proposer: UserId,
        target: Target,
        action: Action,
        commit: PaidCommitment,
        now: Timestamp,
    ) -> Result<Outcome, Error> {
        if !action.is_valid_for(target) {
            return Err(Error::InvalidActionForTarget);
        }
        if commit.amount.is_zero() {
            return Err(Error::ZeroStake);
        }
        if self.balances.get(&proposer).is_zero() {
            return Err(Error::ZeroBalanceCannotOpen);
        }
        if self.proposer_index.contains_key(&proposer) {
            return Err(Error::ProposerHasOpenProposal);
        }
        if self.target_action_index.contains_key(&(target, action)) {
            return Err(Error::DuplicateTargetAction);
        }
        if self.available(&proposer) < commit.amount {
            return Err(Error::InsufficientBalance);
        }
        let proposal = Proposal::new(proposer, target, action, commit, now, self.params());
        let id = proposal.id;
        self.proposer_index.insert(proposer, id);
        self.target_action_index.insert((target, action), id);
        self.proposals.insert(id, proposal);
        Ok(Outcome::ProposalOpened { id })
    }

    fn commit_paid(
        &mut self,
        user: UserId,
        id: ProposalId,
        commit: PaidCommitment,
    ) -> Result<Outcome, Error> {
        if commit.amount.is_zero() {
            return Err(Error::ZeroStake);
        }
        if self.available(&user) < commit.amount {
            return Err(Error::InsufficientBalance);
        }
        let proposal = self.proposals.get_mut(&id).ok_or(Error::ProposalNotFound)?;
        proposal.commit_paid(user, commit)?;
        Ok(Outcome::CommitmentAccepted { proposal: id, user })
    }

    fn commit_free(
        &mut self,
        user: UserId,
        id: ProposalId,
        commit: FreeCommitment,
    ) -> Result<Outcome, Error> {
        if !self.balances.get(&user).is_zero() {
            return Err(Error::BalanceNotZero);
        }
        if self.has_outstanding_free(&user) {
            return Err(Error::OutstandingFreeCommit);
        }
        let proposal = self.proposals.get_mut(&id).ok_or(Error::ProposalNotFound)?;
        proposal.commit_free(user, commit)?;
        Ok(Outcome::CommitmentAccepted { proposal: id, user })
    }

    fn commit_to_reveal(
        &mut self,
        user: UserId,
        id: ProposalId,
        commit: RevealCommit,
    ) -> Result<Outcome, Error> {
        let proposal = self.proposals.get_mut(&id).ok_or(Error::ProposalNotFound)?;
        proposal.commit_to_reveal(user, commit)?;
        Ok(Outcome::RevealCommitted { proposal: id, user })
    }

    fn open(&mut self, user: UserId, id: ProposalId, opening: Opening) -> Result<Outcome, Error> {
        let proposal = self.proposals.get_mut(&id).ok_or(Error::ProposalNotFound)?;
        let direction = proposal.open(user, opening)?;
        Ok(Outcome::Opened {
            proposal: id,
            user,
            direction,
        })
    }

    fn settle(&mut self, id: ProposalId, now: Timestamp) -> Result<Outcome, Error> {
        match self.proposals.get(&id).map(|p| p.phase) {
            None => return Err(Error::ProposalNotFound),
            Some(Phase::AwaitingSettlement) => {}
            Some(_) => return Err(Error::NotYetSettlable),
        }
        let proposal = self.proposals.remove(&id).expect("checked above");

        let mut yes_weight = 0u128;
        let mut no_weight = 0u128;
        let mut opened_paid: Vec<(UserId, Amount, Direction)> = Vec::new();
        let mut forfeitures: Vec<Payout> = Vec::new();
        let mut pot = Amount::ZERO;
        let mut opened_free: Vec<(UserId, Direction)> = Vec::new();

        for (user, c) in &proposal.commitments {
            match (c.kind, c.opened) {
                (crate::proposal::CommitmentKind::Paid { amount }, Some(dir)) => {
                    opened_paid.push((*user, amount, dir));
                    match dir {
                        Direction::Yes => yes_weight = yes_weight.saturating_add(weight(amount)),
                        Direction::No => no_weight = no_weight.saturating_add(weight(amount)),
                    }
                }
                (crate::proposal::CommitmentKind::Paid { amount }, None) => {
                    forfeitures.push(Payout {
                        user: *user,
                        amount,
                    });
                    pot = pot.checked_add_err(amount)?;
                }
                (crate::proposal::CommitmentKind::Free, Some(dir)) => {
                    opened_free.push((*user, dir));
                }
                (crate::proposal::CommitmentKind::Free, None) => {}
            }
        }

        let outcome = decide(
            yes_weight,
            no_weight,
            self.balances.supply(),
            self.params().threshold_for(proposal.action),
        );
        let winning = match outcome {
            OutcomeKind::YesExecuted => Direction::Yes,
            OutcomeKind::NoDefault => Direction::No,
        };

        let mut winners: Vec<(UserId, Amount)> = Vec::new();
        for (user, amount, dir) in &opened_paid {
            if *dir == winning {
                winners.push((*user, *amount));
            } else {
                forfeitures.push(Payout {
                    user: *user,
                    amount: *amount,
                });
                pot = pot.checked_add_err(*amount)?;
            }
        }

        let plan = split_pot(pot, &winners)?;

        let mut free_mints = Vec::new();
        for (user, dir) in opened_free {
            if dir == winning && self.balances.get(&user).is_zero() {
                // Still zero after this settlement's money flows? Free committers
                // are never in forfeitures/payouts, so yes.
                free_mints.push(user);
            }
        }

        let executed_restriction = match (outcome, proposal.action, proposal.target) {
            (OutcomeKind::YesExecuted, Action::Restrict { duration }, Target::User(user)) => {
                let current = self.effective_restriction(&user, now);
                if supersedes(current, duration) {
                    Some(Restriction {
                        user,
                        duration,
                        start: now,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        let prev_hash = self
            .settlements
            .last()
            .map(|e| e.this_hash)
            .unwrap_or_else(|| self.genesis.hash());

        let entry = SettlementEntry::new(
            prev_hash,
            proposal.id,
            proposal.target,
            proposal.action,
            outcome,
            plan.payouts,
            forfeitures,
            free_mints,
            plan.burned,
            executed_restriction,
        );

        if let Err(e) = self.balances.apply_settlement(&entry) {
            self.proposals.insert(id, proposal);
            return Err(e);
        }
        self.proposer_index.remove(&proposal.proposer);
        self.target_action_index
            .remove(&(proposal.target, proposal.action));
        self.settlements.push(entry.clone());
        Ok(Outcome::Settled(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitment::{Opening, Salt};
    use crate::types::{DurationSecs, MessageId};

    fn uid(n: u8) -> UserId {
        UserId::from_byte(n)
    }

    fn room() -> Room {
        Room::genesis(Genesis::new(vec![uid(1), uid(2), uid(3)], RoomParams::defaults()).unwrap())
            .unwrap()
    }

    fn open_remove(
        room: &mut Room,
        proposer: UserId,
        dir: Direction,
        now: Timestamp,
    ) -> ProposalId {
        let opening = Opening::new(dir, Salt::from_byte(proposer.0[0]));
        let commit = opening.paid_commitment(Amount::TOKEN);
        match room
            .apply(
                Command::OpenProposal {
                    proposer,
                    target: Target::Message(MessageId::from_byte(9)),
                    action: Action::RemoveMessage,
                    commit,
                },
                now,
            )
            .unwrap()
        {
            Outcome::ProposalOpened { id } => id,
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn zero_balance_cannot_open() {
        let mut r = room();
        let opening = Opening::new(Direction::Yes, Salt::from_byte(1));
        let err = r
            .apply(
                Command::OpenProposal {
                    proposer: uid(9),
                    target: Target::Message(MessageId::from_byte(1)),
                    action: Action::RemoveMessage,
                    commit: opening.paid_commitment(Amount::TOKEN),
                },
                Timestamp::from_millis(0),
            )
            .unwrap_err();
        assert_eq!(err, Error::ZeroBalanceCannotOpen);
    }

    #[test]
    fn one_proposal_per_proposer() {
        let mut r = room();
        let now = Timestamp::from_millis(0);
        open_remove(&mut r, uid(1), Direction::Yes, now);
        let opening = Opening::new(Direction::Yes, Salt::from_byte(2));
        let err = r
            .apply(
                Command::OpenProposal {
                    proposer: uid(1),
                    target: Target::Message(MessageId::from_byte(8)),
                    action: Action::RemoveMessage,
                    commit: opening.paid_commitment(Amount::TOKEN),
                },
                now,
            )
            .unwrap_err();
        assert_eq!(err, Error::ProposerHasOpenProposal);
    }

    #[test]
    fn duplicate_target_action_rejected() {
        let mut r = room();
        let now = Timestamp::from_millis(0);
        open_remove(&mut r, uid(1), Direction::Yes, now);
        let opening = Opening::new(Direction::No, Salt::from_byte(2));
        let err = r
            .apply(
                Command::OpenProposal {
                    proposer: uid(2),
                    target: Target::Message(MessageId::from_byte(9)),
                    action: Action::RemoveMessage,
                    commit: opening.paid_commitment(Amount::TOKEN),
                },
                now,
            )
            .unwrap_err();
        assert_eq!(err, Error::DuplicateTargetAction);
    }

    #[test]
    fn different_durations_on_same_user_ok() {
        let mut r = room();
        let now = Timestamp::from_millis(0);
        let o1 = Opening::new(Direction::Yes, Salt::from_byte(1));
        r.apply(
            Command::OpenProposal {
                proposer: uid(1),
                target: Target::User(uid(9)),
                action: Action::Restrict {
                    duration: DurationSecs::from_secs(60),
                },
                commit: o1.paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap();
        let o2 = Opening::new(Direction::Yes, Salt::from_byte(2));
        r.apply(
            Command::OpenProposal {
                proposer: uid(2),
                target: Target::User(uid(9)),
                action: Action::Restrict {
                    duration: DurationSecs::from_secs(600),
                },
                commit: o2.paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap();
        assert_eq!(r.open_proposals().count(), 2);
    }

    #[test]
    fn available_subtracts_locks() {
        let mut r = room();
        open_remove(&mut r, uid(1), Direction::Yes, Timestamp::from_millis(0));
        assert_eq!(r.available(&uid(1)), Amount::ZERO);
        assert_eq!(r.balance(&uid(1)), Amount::TOKEN);
    }
}

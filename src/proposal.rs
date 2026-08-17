//! Ephemeral proposal state. Deleted on settlement.

use std::collections::BTreeMap;

use crate::amount::Amount;
use crate::commitment::{
    FreeCommitment, Opening, PaidCommitment, RevealCommit, verify_commit, verify_reveal,
};
use crate::crypto::hash_tagged;
use crate::encode::TAG_PROPOSAL;
use crate::error::Error;
use crate::params::RoomParams;
use crate::types::{
    Action, Direction, Hash, Phase, ProposalId, Target, Timestamp, UserId, encode_action,
    encode_target,
};

/// Kind of first-round commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitmentKind {
    Paid { amount: Amount },
    Free,
}

/// Per-user commitment progress on one proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserCommitment {
    pub hash: Hash,
    pub kind: CommitmentKind,
    pub reveal_hash: Option<Hash>,
    pub opened: Option<Direction>,
}

impl UserCommitment {
    pub fn locked(&self) -> Amount {
        match self.kind {
            CommitmentKind::Paid { amount } => amount,
            CommitmentKind::Free => Amount::ZERO,
        }
    }

    pub fn is_free(&self) -> bool {
        matches!(self.kind, CommitmentKind::Free)
    }
}

/// In-flight proposal. Not part of persistent state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    pub id: ProposalId,
    pub proposer: UserId,
    pub target: Target,
    pub action: Action,
    pub opened_at: Timestamp,
    pub betting_deadline: Timestamp,
    pub reveal_deadline: Timestamp,
    pub opening_deadline: Timestamp,
    pub phase: Phase,
    pub commitments: BTreeMap<UserId, UserCommitment>,
}

impl Proposal {
    pub fn new(
        proposer: UserId,
        target: Target,
        action: Action,
        first: PaidCommitment,
        now: Timestamp,
        params: &RoomParams,
    ) -> Self {
        let id = proposal_id(proposer, target, action, first.hash);
        let betting_deadline = now.saturating_add_millis(params.betting_window_ms);
        let reveal_deadline =
            betting_deadline.saturating_add_millis(params.commit_to_reveal_window_ms);
        let opening_deadline = reveal_deadline.saturating_add_millis(params.opening_window_ms);
        let mut commitments = BTreeMap::new();
        commitments.insert(
            proposer,
            UserCommitment {
                hash: first.hash,
                kind: CommitmentKind::Paid {
                    amount: first.amount,
                },
                reveal_hash: None,
                opened: None,
            },
        );
        Self {
            id,
            proposer,
            target,
            action,
            opened_at: now,
            betting_deadline,
            reveal_deadline,
            opening_deadline,
            phase: Phase::Betting,
            commitments,
        }
    }

    pub fn advance(&mut self, now: Timestamp) {
        self.phase = phase_at(
            now,
            self.betting_deadline,
            self.reveal_deadline,
            self.opening_deadline,
        );
    }

    pub fn locked_by(&self, user: &UserId) -> Amount {
        self.commitments
            .get(user)
            .map(UserCommitment::locked)
            .unwrap_or(Amount::ZERO)
    }

    pub fn require_phase(&self, expected: Phase) -> Result<(), Error> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(Error::WrongPhase {
                expected,
                actual: self.phase,
            })
        }
    }

    pub fn commit_paid(&mut self, user: UserId, commit: PaidCommitment) -> Result<(), Error> {
        self.require_phase(Phase::Betting)?;
        if commit.amount.is_zero() {
            return Err(Error::ZeroStake);
        }
        if self.commitments.contains_key(&user) {
            return Err(Error::AlreadyCommitted);
        }
        self.commitments.insert(
            user,
            UserCommitment {
                hash: commit.hash,
                kind: CommitmentKind::Paid {
                    amount: commit.amount,
                },
                reveal_hash: None,
                opened: None,
            },
        );
        Ok(())
    }

    pub fn commit_free(&mut self, user: UserId, commit: FreeCommitment) -> Result<(), Error> {
        self.require_phase(Phase::Betting)?;
        if self.commitments.contains_key(&user) {
            return Err(Error::AlreadyCommitted);
        }
        self.commitments.insert(
            user,
            UserCommitment {
                hash: commit.hash,
                kind: CommitmentKind::Free,
                reveal_hash: None,
                opened: None,
            },
        );
        Ok(())
    }

    pub fn commit_to_reveal(&mut self, user: UserId, reveal: RevealCommit) -> Result<(), Error> {
        self.require_phase(Phase::CommitToReveal)?;
        let c = self.commitments.get_mut(&user).ok_or(Error::NoCommitment)?;
        if c.reveal_hash.is_some() {
            return Err(Error::AlreadyRevealCommitted);
        }
        c.reveal_hash = Some(reveal.hash);
        Ok(())
    }

    pub fn open(&mut self, user: UserId, opening: Opening) -> Result<Direction, Error> {
        self.require_phase(Phase::Opening)?;
        let c = self.commitments.get_mut(&user).ok_or(Error::NoCommitment)?;
        if c.opened.is_some() {
            return Err(Error::AlreadyOpened);
        }
        let reveal = c.reveal_hash.ok_or(Error::NoRevealCommit)?;
        let amount = c.locked();
        if !verify_commit(c.hash, amount, &opening) || !verify_reveal(reveal, c.hash, &opening) {
            return Err(Error::OpeningVerifyFailed);
        }
        c.opened = Some(opening.direction);
        Ok(opening.direction)
    }
}

pub fn proposal_id(
    proposer: UserId,
    target: Target,
    action: Action,
    first_commit: Hash,
) -> ProposalId {
    let hash = hash_tagged(TAG_PROPOSAL, |e| {
        e.bytes32(proposer.as_bytes());
        encode_target(e, target);
        encode_action(e, action);
        e.bytes32(first_commit.as_bytes());
    });
    ProposalId::from_bytes(*hash.as_bytes())
}

pub fn phase_at(
    now: Timestamp,
    betting_deadline: Timestamp,
    reveal_deadline: Timestamp,
    opening_deadline: Timestamp,
) -> Phase {
    if now < betting_deadline {
        Phase::Betting
    } else if now < reveal_deadline {
        Phase::CommitToReveal
    } else if now < opening_deadline {
        Phase::Opening
    } else {
        Phase::AwaitingSettlement
    }
}

/// Public snapshot. Never includes direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalView {
    pub id: ProposalId,
    pub proposer: UserId,
    pub target: Target,
    pub action: Action,
    pub phase: Phase,
    pub opened_at: Timestamp,
    pub betting_deadline: Timestamp,
    pub reveal_deadline: Timestamp,
    pub opening_deadline: Timestamp,
    pub commitment_count: usize,
    pub total_locked: Amount,
}

impl Proposal {
    pub fn view(&self) -> ProposalView {
        let total_locked = self
            .commitments
            .values()
            .fold(Amount::ZERO, |acc, c| acc.saturating_add(c.locked()));
        ProposalView {
            id: self.id,
            proposer: self.proposer,
            target: self.target,
            action: self.action,
            phase: self.phase,
            opened_at: self.opened_at,
            betting_deadline: self.betting_deadline,
            reveal_deadline: self.reveal_deadline,
            opening_deadline: self.opening_deadline,
            commitment_count: self.commitments.len(),
            total_locked,
        }
    }
}

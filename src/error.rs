//! Typed errors returned by the state machine.

use crate::types::Phase;

/// Recoverable rejection or integrity failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("genesis must list at least one founding member")]
    GenesisEmpty,

    #[error("genesis lists a founder more than once")]
    DuplicateFounder,

    #[error("action is not valid for this target")]
    InvalidActionForTarget,

    #[error("paid commitment amount must be greater than zero")]
    ZeroStake,

    #[error("insufficient available balance")]
    InsufficientBalance,

    #[error("only users with a strictly positive balance may open a proposal")]
    ZeroBalanceCannotOpen,

    #[error("proposer already has an open proposal")]
    ProposerHasOpenProposal,

    #[error("an open proposal already exists for this (target, action) pair")]
    DuplicateTargetAction,

    #[error("proposal not found")]
    ProposalNotFound,

    #[error("command is not valid in phase {actual:?} (expected {expected:?})")]
    WrongPhase { expected: Phase, actual: Phase },

    #[error("user already has a commitment on this proposal")]
    AlreadyCommitted,

    #[error("user already posted a commit-to-reveal")]
    AlreadyRevealCommitted,

    #[error("user already opened this commitment")]
    AlreadyOpened,

    #[error("user already has an outstanding free commitment")]
    OutstandingFreeCommit,

    #[error("free commitments are only allowed while balance is exactly zero")]
    BalanceNotZero,

    #[error("opening failed commitment verification")]
    OpeningVerifyFailed,

    #[error("user has no commitment on this proposal")]
    NoCommitment,

    #[error("user has not posted a commit-to-reveal")]
    NoRevealCommit,

    #[error("settlement is only allowed after the opening deadline")]
    NotYetSettlable,

    #[error("settlement log hash chain is broken")]
    BrokenHashChain,

    #[error("settlement entry hash does not match its body")]
    SettlementHashMismatch,

    #[error("arithmetic overflow")]
    Overflow,
}

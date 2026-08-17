//! Host-attributed commands and the outcomes `Room::apply` returns.

use crate::commitment::{FreeCommitment, Opening, PaidCommitment, RevealCommit};
use crate::settlement::SettlementEntry;
use crate::types::{Action, Direction, Phase, ProposalId, Target, UserId};

/// Authenticated (by the host) input to [`crate::Room::apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// First paid commitment. Creates the ephemeral proposal.
    OpenProposal {
        proposer: UserId,
        target: Target,
        action: Action,
        commit: PaidCommitment,
    },
    /// Additional paid commitment during the betting window.
    CommitPaid {
        user: UserId,
        proposal: ProposalId,
        commit: PaidCommitment,
    },
    /// Zero-balance directional commitment. No lock.
    CommitFree {
        user: UserId,
        proposal: ProposalId,
        commit: FreeCommitment,
    },
    /// Second-round hash during the commit-to-reveal window.
    CommitToReveal {
        user: UserId,
        proposal: ProposalId,
        commit: RevealCommit,
    },
    /// Publish the preimage during the opening window.
    Open {
        user: UserId,
        proposal: ProposalId,
        opening: Opening,
    },
    /// Permissionless. Legal only after the opening deadline.
    Settle { proposal: ProposalId },
}

/// What a successful `apply` or `tick` did.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // `Settled` is the value hosts persist
pub enum Outcome {
    ProposalOpened {
        id: ProposalId,
    },
    CommitmentAccepted {
        proposal: ProposalId,
        user: UserId,
    },
    RevealCommitted {
        proposal: ProposalId,
        user: UserId,
    },
    Opened {
        proposal: ProposalId,
        user: UserId,
        direction: Direction,
    },
    PhaseChanged {
        proposal: ProposalId,
        phase: Phase,
    },
    Settled(SettlementEntry),
}

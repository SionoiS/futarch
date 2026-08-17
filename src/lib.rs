//! Deterministic futarchy gadget for decentralized chat moderation.
//!
//! This crate is a transport-agnostic state machine. A host authenticates users,
//! supplies a clock, gossips [`Command`]s, and applies recorded consensus
//! (hide a message, refuse to relay from a restricted user).
//!
//! Tokens are room-local. Balances are **derived** by replaying the genesis
//! record plus an append-only settlement log. Ephemeral proposal state
//! (commitments, locks, timers) is deleted on settlement.
//!
//! # Host integration
//!
//! ```rust
//! use futarch::{
//!     Action, Amount, Command, Direction, Genesis, MessageId, Opening, Room,
//!     RoomParams, Salt, Target, Timestamp, UserId,
//! };
//!
//! # fn main() -> Result<(), futarch::Error> {
//! let alice = UserId::from_byte(1);
//! let bob = UserId::from_byte(2);
//! let mut room = Room::genesis(Genesis::new(
//!     vec![alice, bob],
//!     RoomParams::defaults(),
//! )?)?;
//!
//! let opening = Opening::new(Direction::Yes, Salt::from_byte(7));
//! room.apply(
//!     Command::OpenProposal {
//!         proposer: alice,
//!         target: Target::Message(MessageId::from_byte(9)),
//!         action: Action::RemoveMessage,
//!         commit: opening.paid_commitment(Amount::TOKEN),
//!     },
//!     Timestamp::from_millis(1_000),
//! )?;
//! # let _ = (bob, room);
//! # Ok(())
//! # }
//! ```
//!
//! # Time
//!
//! [`Timestamp`] is Unix milliseconds supplied by the host. Window deadlines
//! are absolute timestamps computed when a proposal opens. Hosts that disagree
//! on time can disagree on settlement; agree on a clock (or a logical tick)
//! outside this crate.
//!
//! # Persistence
//!
//! Persist [`Genesis`] and [`SettlementEntry`] values. [`Room::from_log`]
//! rebuilds balances and restrictions. Open proposals are ephemeral and are
//! **not** in the log — persist them yourself if the process can restart
//! mid-proposal.
//!
//! The design documents in `docs/` are the source of truth for the rules.

#![forbid(unsafe_code)]

mod amount;
mod balances;
mod command;
mod commitment;
mod crypto;
mod encode;
mod error;
mod genesis;
mod params;
mod payout;
mod proposal;
mod restriction;
mod room;
mod settlement;
mod threshold;
mod types;

pub use amount::{Amount, TOKEN};
pub use command::{Command, Outcome};
pub use commitment::{
    FreeCommitment, Opening, PaidCommitment, RevealCommit, Salt, commit_hash, reveal_hash,
};
pub use error::Error;
pub use genesis::Genesis;
pub use params::{
    DEFAULT_BETTING_WINDOW_MS, DEFAULT_COMMIT_TO_REVEAL_WINDOW_MS, DEFAULT_OPENING_WINDOW_MS,
    Ratio, RoomParams, SeverityClass, SeverityThreshold,
};
pub use proposal::ProposalView;
pub use restriction::Restriction;
pub use room::Room;
pub use settlement::{OutcomeKind, Payout, SettlementEntry};
pub use types::{
    Action, Direction, DurationSecs, Hash, MessageId, Phase, ProposalId, Target, Timestamp, UserId,
};

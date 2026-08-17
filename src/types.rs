//! Opaque identifiers and protocol enumerations.

use crate::encode::Encoder;

macro_rules! byte_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub [u8; 32]);

        impl $name {
            /// Wrap a raw 32-byte value.
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Convenience constructor for tests and examples: byte 0 is `n`, rest zero.
            pub const fn from_byte(n: u8) -> Self {
                let mut bytes = [0u8; 32];
                bytes[0] = n;
                Self(bytes)
            }

            /// Borrow the raw bytes.
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
    };
}

byte_id!(
    /// Host-assigned participant identity. This crate does not authenticate it.
    UserId
);
byte_id!(
    /// Hash (or other 32-byte reference) of a chat message, supplied by the host.
    MessageId
);
byte_id!(
    /// Deterministic proposal identifier: hash of proposer, target, action, first commit.
    ProposalId
);
byte_id!(
    /// SHA-256 digest.
    Hash
);

/// Unix time in milliseconds, supplied by the host on every `apply` / `tick`.
///
/// Hosts that disagree on time can disagree on window close and settlement.
/// That is an integration concern, not something this crate solves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Construct from Unix milliseconds.
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    /// Unix milliseconds.
    pub const fn millis(self) -> u64 {
        self.0
    }

    /// Saturating add of a millisecond delta.
    pub const fn saturating_add_millis(self, delta: u64) -> Self {
        Self(self.0.saturating_add(delta))
    }
}

/// Restriction duration in **seconds**. [`DurationSecs::PERMANENT`] never expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationSecs(u64);

impl DurationSecs {
    /// Permanent restriction (`u64::MAX` seconds).
    pub const PERMANENT: Self = Self(u64::MAX);

    /// Finite duration in seconds.
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    /// Seconds (or `u64::MAX` if permanent).
    pub const fn secs(self) -> u64 {
        self.0
    }

    /// `true` for the permanent sentinel.
    pub const fn is_permanent(self) -> bool {
        self.0 == u64::MAX
    }
}

/// Lifecycle of an ephemeral proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Direction-secret, amount-public betting.
    Betting,
    /// Second commitment to the opening; directions still hidden.
    CommitToReveal,
    /// Preimages published; settlement waits for the deadline.
    Opening,
    /// Opening window closed; [`crate::Command::Settle`] is legal.
    AwaitingSettlement,
}

/// Secret (until opening) side of a bet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Oppose the action. Encoded as `0`.
    No = 0,
    /// Support the action. Encoded as `1`.
    Yes = 1,
}

impl Direction {
    pub(crate) const fn wire(self) -> u8 {
        self as u8
    }
}

/// Proposed moderation action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Remove a message. Only valid with [`Target::Message`].
    RemoveMessage,
    /// Restrict a user from sending for `duration`. Only valid with [`Target::User`].
    Restrict { duration: DurationSecs },
}

/// What the proposal is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    /// A chat message, identified by the host's message hash.
    Message(MessageId),
    /// A participant.
    User(UserId),
}

impl Action {
    /// Message actions only pair with message targets; restrictions only with users.
    pub const fn is_valid_for(self, target: Target) -> bool {
        matches!(
            (self, target),
            (Action::RemoveMessage, Target::Message(_))
                | (Action::Restrict { .. }, Target::User(_))
        )
    }
}

pub(crate) fn encode_target(encoder: &mut Encoder, target: Target) {
    match target {
        Target::Message(id) => {
            encoder.u8(0);
            encoder.bytes32(id.as_bytes());
        }
        Target::User(id) => {
            encoder.u8(1);
            encoder.bytes32(id.as_bytes());
        }
    }
}

pub(crate) fn encode_action(encoder: &mut Encoder, action: Action) {
    match action {
        Action::RemoveMessage => {
            encoder.u8(0);
        }
        Action::Restrict { duration } => {
            encoder.u8(1);
            encoder.u64(duration.secs());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_target_pairing() {
        let msg = Target::Message(MessageId::from_byte(1));
        let user = Target::User(UserId::from_byte(2));
        assert!(Action::RemoveMessage.is_valid_for(msg));
        assert!(!Action::RemoveMessage.is_valid_for(user));
        assert!(
            Action::Restrict {
                duration: DurationSecs::from_secs(60)
            }
            .is_valid_for(user)
        );
        assert!(
            !Action::Restrict {
                duration: DurationSecs::PERMANENT
            }
            .is_valid_for(msg)
        );
    }

    #[test]
    fn timestamp_saturating_add() {
        let t = Timestamp::from_millis(u64::MAX - 5);
        assert_eq!(t.saturating_add_millis(10).millis(), u64::MAX);
    }
}

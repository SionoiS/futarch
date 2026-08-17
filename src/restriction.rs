//! Derived restrictions: later longer `D` overrides and restarts the clock.

use crate::settlement::SettlementEntry;
use crate::types::{DurationSecs, Timestamp, UserId};

/// An executed user restriction. Permanent never expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Restriction {
    pub user: UserId,
    pub duration: DurationSecs,
    pub start: Timestamp,
}

impl Restriction {
    /// Active at `now` (half-open: expires when `now >= start + D`).
    pub fn is_active(self, now: Timestamp) -> bool {
        if self.duration.is_permanent() {
            return true;
        }
        let dur_ms = self.duration.secs().saturating_mul(1_000);
        now.millis() < self.start.millis().saturating_add(dur_ms)
    }
}

/// Effective restriction on `user` at `now`: the longest still-active executed
/// restriction, ties broken by later start.
pub fn effective_restriction<'a>(
    settlements: impl IntoIterator<Item = &'a SettlementEntry>,
    user: &UserId,
    now: Timestamp,
) -> Option<Restriction> {
    let mut best: Option<Restriction> = None;
    for s in settlements {
        let Some(r) = s.executed_restriction else {
            continue;
        };
        if r.user != *user || !r.is_active(now) {
            continue;
        }
        let take = match best {
            None => true,
            Some(b) => r.duration > b.duration || (r.duration == b.duration && r.start > b.start),
        };
        if take {
            best = Some(r);
        }
    }
    best
}

/// Whether a newly passing restriction of `duration` should become effective
/// given the currently active one (if any).
pub fn supersedes(current: Option<Restriction>, duration: DurationSecs) -> bool {
    match current {
        None => true,
        Some(c) => duration > c.duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::Amount;
    use crate::settlement::{OutcomeKind, SettlementEntry};
    use crate::types::{Action, Hash, ProposalId, Target};

    fn entry(r: Option<Restriction>) -> SettlementEntry {
        SettlementEntry::new(
            Hash::from_byte(1),
            ProposalId::from_byte(2),
            Target::User(UserId::from_byte(1)),
            Action::Restrict {
                duration: r.map(|x| x.duration).unwrap_or(DurationSecs::from_secs(1)),
            },
            OutcomeKind::YesExecuted,
            vec![],
            vec![],
            vec![],
            Amount::ZERO,
            r,
        )
    }

    #[test]
    fn longer_later_overrides() {
        let user = UserId::from_byte(1);
        let short = Restriction {
            user,
            duration: DurationSecs::from_secs(60),
            start: Timestamp::from_millis(0),
        };
        let long = Restriction {
            user,
            duration: DurationSecs::from_secs(600),
            start: Timestamp::from_millis(10_000),
        };
        let log = [entry(Some(short)), entry(Some(long))];
        let now = Timestamp::from_millis(11_000);
        let got = effective_restriction(&log, &user, now).unwrap();
        assert_eq!(got.duration, long.duration);
        assert_eq!(got.start, long.start);
    }

    #[test]
    fn shorter_does_not_override() {
        assert!(!supersedes(
            Some(Restriction {
                user: UserId::from_byte(1),
                duration: DurationSecs::from_secs(600),
                start: Timestamp::from_millis(0),
            }),
            DurationSecs::from_secs(60)
        ));
    }

    #[test]
    fn expiry_clears() {
        let user = UserId::from_byte(1);
        let r = Restriction {
            user,
            duration: DurationSecs::from_secs(1),
            start: Timestamp::from_millis(0),
        };
        let log = [entry(Some(r))];
        assert!(effective_restriction(&log, &user, Timestamp::from_millis(999)).is_some());
        assert!(effective_restriction(&log, &user, Timestamp::from_millis(1_000)).is_none());
    }

    #[test]
    fn permanent_never_expires() {
        let user = UserId::from_byte(1);
        let r = Restriction {
            user,
            duration: DurationSecs::PERMANENT,
            start: Timestamp::from_millis(0),
        };
        assert!(r.is_active(Timestamp::from_millis(u64::MAX)));
    }
}

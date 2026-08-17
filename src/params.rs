//! Room parameters, frozen at genesis, and the shipped defaults.

use crate::types::{Action, DurationSecs};

/// Milliseconds in one second / minute / hour / day, for window and bucket defaults.
pub const MS_PER_SEC: u64 = 1_000;
pub const MS_PER_MIN: u64 = 60 * MS_PER_SEC;
pub const SECS_PER_HOUR: u64 = 3_600;
pub const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;

/// Default windows (chat-scale, still long enough for a mobile two-step reveal).
pub const DEFAULT_BETTING_WINDOW_MS: u64 = 10 * MS_PER_MIN;
pub const DEFAULT_COMMIT_TO_REVEAL_WINDOW_MS: u64 = 3 * MS_PER_MIN;
pub const DEFAULT_OPENING_WINDOW_MS: u64 = MS_PER_MIN;

/// Default duration-bucket edges (restriction `D` is in seconds).
pub const DEFAULT_SHORT_MAX_SECS: u64 = SECS_PER_HOUR;
pub const DEFAULT_MEDIUM_MAX_SECS: u64 = SECS_PER_DAY;
pub const DEFAULT_LONG_MAX_SECS: u64 = 7 * SECS_PER_DAY;

/// Rational yes/no margin: execute only if `W_yes * den >= W_no * num`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ratio {
    /// Numerator (e.g. 133 for 1.33×).
    pub numerator: u64,
    /// Denominator (e.g. 100 for 1.33×).
    pub denominator: u64,
}

impl Ratio {
    /// `numerator / denominator`. Denominator must be non-zero; not checked here
    /// because genesis construction is the only writer.
    pub const fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// Threshold for one severity class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SeverityThreshold {
    /// Required `W_yes / W_no` margin.
    pub ratio: Ratio,
    /// Floor as basis points of `isqrt(supply)`. 500 = 5%.
    ///
    /// `W_yes` is in weight space (`isqrt` of subunits), so the floor is
    /// `isqrt(supply) * floor_bps / 10_000`, not a raw-stake percentage.
    pub floor_bps: u64,
}

/// Severity class used to pick a threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeverityClass {
    MessageRemove,
    ShortRestrict,
    MediumRestrict,
    LongRestrict,
    PermanentRestrict,
}

/// Frozen at room creation. The chat creator may override defaults once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomParams {
    pub remove_message: SeverityThreshold,
    pub short_restrict: SeverityThreshold,
    pub medium_restrict: SeverityThreshold,
    pub long_restrict: SeverityThreshold,
    pub permanent_restrict: SeverityThreshold,
    /// Inclusive max duration (seconds) classified as short.
    pub short_max: DurationSecs,
    /// Inclusive max duration classified as medium.
    pub medium_max: DurationSecs,
    /// Inclusive max duration classified as long. Above this (and not permanent)
    /// is also treated as long; permanent is its own class.
    pub long_max: DurationSecs,
    /// Betting window length, milliseconds.
    pub betting_window_ms: u64,
    /// Commit-to-reveal window length, milliseconds.
    pub commit_to_reveal_window_ms: u64,
    /// Opening window length, milliseconds.
    pub opening_window_ms: u64,
}

impl RoomParams {
    /// Shipped defaults from the implementation plan.
    pub const fn defaults() -> Self {
        Self {
            remove_message: SeverityThreshold {
                ratio: Ratio::new(133, 100),
                floor_bps: 500,
            },
            short_restrict: SeverityThreshold {
                ratio: Ratio::new(133, 100),
                floor_bps: 500,
            },
            medium_restrict: SeverityThreshold {
                ratio: Ratio::new(150, 100),
                floor_bps: 800,
            },
            long_restrict: SeverityThreshold {
                ratio: Ratio::new(175, 100),
                floor_bps: 1200,
            },
            permanent_restrict: SeverityThreshold {
                ratio: Ratio::new(200, 100),
                floor_bps: 2000,
            },
            short_max: DurationSecs::from_secs(DEFAULT_SHORT_MAX_SECS),
            medium_max: DurationSecs::from_secs(DEFAULT_MEDIUM_MAX_SECS),
            long_max: DurationSecs::from_secs(DEFAULT_LONG_MAX_SECS),
            betting_window_ms: DEFAULT_BETTING_WINDOW_MS,
            commit_to_reveal_window_ms: DEFAULT_COMMIT_TO_REVEAL_WINDOW_MS,
            opening_window_ms: DEFAULT_OPENING_WINDOW_MS,
        }
    }

    /// Classify an action into a severity bucket.
    pub const fn classify(&self, action: Action) -> SeverityClass {
        match action {
            Action::RemoveMessage => SeverityClass::MessageRemove,
            Action::Restrict { duration } => {
                if duration.is_permanent() {
                    SeverityClass::PermanentRestrict
                } else if duration.secs() <= self.short_max.secs() {
                    SeverityClass::ShortRestrict
                } else if duration.secs() <= self.medium_max.secs() {
                    SeverityClass::MediumRestrict
                } else {
                    SeverityClass::LongRestrict
                }
            }
        }
    }

    /// Threshold used for `action`.
    pub const fn threshold_for(&self, action: Action) -> SeverityThreshold {
        match self.classify(action) {
            SeverityClass::MessageRemove => self.remove_message,
            SeverityClass::ShortRestrict => self.short_restrict,
            SeverityClass::MediumRestrict => self.medium_restrict,
            SeverityClass::LongRestrict => self.long_restrict,
            SeverityClass::PermanentRestrict => self.permanent_restrict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_duration_buckets() {
        let p = RoomParams::defaults();
        assert_eq!(
            p.classify(Action::RemoveMessage),
            SeverityClass::MessageRemove
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::from_secs(SECS_PER_HOUR)
            }),
            SeverityClass::ShortRestrict
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::from_secs(SECS_PER_HOUR + 1)
            }),
            SeverityClass::MediumRestrict
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::from_secs(SECS_PER_DAY)
            }),
            SeverityClass::MediumRestrict
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::from_secs(SECS_PER_DAY + 1)
            }),
            SeverityClass::LongRestrict
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::from_secs(7 * SECS_PER_DAY)
            }),
            SeverityClass::LongRestrict
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::from_secs(7 * SECS_PER_DAY + 1)
            }),
            SeverityClass::LongRestrict
        );
        assert_eq!(
            p.classify(Action::Restrict {
                duration: DurationSecs::PERMANENT
            }),
            SeverityClass::PermanentRestrict
        );
    }

    #[test]
    fn defaults_match_plan_table() {
        let p = RoomParams::defaults();
        assert_eq!(p.remove_message.ratio.numerator, 133);
        assert_eq!(p.remove_message.floor_bps, 500);
        assert_eq!(p.medium_restrict.ratio.numerator, 150);
        assert_eq!(p.medium_restrict.floor_bps, 800);
        assert_eq!(p.long_restrict.ratio.numerator, 175);
        assert_eq!(p.permanent_restrict.ratio.numerator, 200);
        assert_eq!(p.permanent_restrict.floor_bps, 2000);
        assert_eq!(p.betting_window_ms, 10 * MS_PER_MIN);
        assert_eq!(p.commit_to_reveal_window_ms, 3 * MS_PER_MIN);
        assert_eq!(p.opening_window_ms, DEFAULT_OPENING_WINDOW_MS);
    }
}

//! Integer √S weights and the yes-vs-no + floor decision.

use crate::amount::Amount;
use crate::params::SeverityThreshold;
use crate::settlement::OutcomeKind;

/// Effective threshold weight of a full stake: `isqrt(subunits)`.
pub fn weight(amount: Amount) -> u128 {
    u128::from(amount.subunits().isqrt())
}

/// Floor in weight space: `isqrt(supply) * floor_bps / 10_000`.
pub fn floor_weight(supply: Amount, floor_bps: u64) -> u128 {
    weight(supply).saturating_mul(u128::from(floor_bps)) / 10_000
}

/// Execute iff `W_yes >= ratio * W_no` and `W_yes` clears the supply-relative floor.
pub fn decide(
    yes_weight: u128,
    no_weight: u128,
    supply: Amount,
    threshold: SeverityThreshold,
) -> OutcomeKind {
    let den = u128::from(threshold.ratio.denominator);
    let num = u128::from(threshold.ratio.numerator);
    // den == 0 would make every comparison false; treat as no-win.
    let yes_clears_ratio =
        den != 0 && yes_weight.saturating_mul(den) >= no_weight.saturating_mul(num);
    let yes_clears_floor = yes_weight >= floor_weight(supply, threshold.floor_bps);
    if yes_clears_ratio && yes_clears_floor {
        OutcomeKind::YesExecuted
    } else {
        OutcomeKind::NoDefault
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::RoomParams;
    use crate::types::Action;

    #[test]
    fn isqrt_of_one_token_is_1000() {
        assert_eq!(weight(Amount::TOKEN), 1000);
        assert_eq!(weight(Amount::from_tokens(9).unwrap()), 3000);
    }

    #[test]
    fn just_below_ratio_defaults_to_no() {
        let t = RoomParams::defaults().threshold_for(Action::RemoveMessage);
        // 1.32× < 1.33× : 1320 vs 1000, 1320*100 = 132000 < 1000*133 = 133000
        let supply = Amount::from_tokens(10).unwrap();
        assert_eq!(decide(1320, 1000, supply, t), OutcomeKind::NoDefault);
        assert_eq!(decide(1330, 1000, supply, t), OutcomeKind::YesExecuted);
    }

    #[test]
    fn ratio_met_but_floor_missed_defaults_to_no() {
        let t = RoomParams::defaults().threshold_for(Action::RemoveMessage);
        // Large supply → high floor. isqrt(1e12) = 1e6, 5% = 50_000.
        let supply = Amount::from_subunits(1_000_000_000_000);
        assert_eq!(decide(2000, 0, supply, t), OutcomeKind::NoDefault);
        assert_eq!(decide(50_000, 0, supply, t), OutcomeKind::YesExecuted);
    }

    #[test]
    fn whale_weight_is_concave() {
        let whale = weight(Amount::from_tokens(9).unwrap());
        let minnow = weight(Amount::TOKEN);
        assert_eq!(whale, 3 * minnow);
        assert_ne!(whale, 9 * minnow);
    }

    #[test]
    fn severity_picks_stricter_permanent_bar() {
        let p = RoomParams::defaults();
        let supply = Amount::from_tokens(10).unwrap();
        // 1.33× passes message-remove, fails permanent (needs 2.00×).
        let yes = 1400;
        let no = 1000;
        assert_eq!(
            decide(yes, no, supply, p.threshold_for(Action::RemoveMessage)),
            OutcomeKind::YesExecuted
        );
        assert_eq!(
            decide(
                yes,
                no,
                supply,
                p.threshold_for(Action::Restrict {
                    duration: crate::types::DurationSecs::PERMANENT
                })
            ),
            OutcomeKind::NoDefault
        );
    }
}

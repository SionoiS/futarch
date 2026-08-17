//! Pro-rata pot split on full opened winning stake. Remainder burned.

use crate::amount::Amount;
use crate::error::Error;
use crate::settlement::Payout;
use crate::types::UserId;

/// Result of splitting `pot` among winning opened stakes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayoutPlan {
    pub payouts: Vec<Payout>,
    pub burned: Amount,
}

/// `floor(pot * stake_i / total_winning_stake)`. Empty winner or zero pot → full burn.
///
/// Free committers are not passed in; they never share the pot.
pub fn split_pot(pot: Amount, winners: &[(UserId, Amount)]) -> Result<PayoutPlan, Error> {
    let total: u128 = winners.iter().map(|(_, a)| u128::from(a.subunits())).sum();
    if total == 0 || pot.is_zero() {
        return Ok(PayoutPlan {
            payouts: Vec::new(),
            burned: pot,
        });
    }
    let pot_n = u128::from(pot.subunits());
    let mut payouts = Vec::new();
    let mut distributed = 0u128;
    for (user, stake) in winners {
        let share = pot_n
            .checked_mul(u128::from(stake.subunits()))
            .ok_or(Error::Overflow)?
            / total;
        if share > 0 {
            let share_u64 = u64::try_from(share).map_err(|_| Error::Overflow)?;
            payouts.push(Payout {
                user: *user,
                amount: Amount::from_subunits(share_u64),
            });
            distributed = distributed.saturating_add(share);
        }
    }
    let burned = u64::try_from(pot_n - distributed).map_err(|_| Error::Overflow)?;
    Ok(PayoutPlan {
        payouts,
        burned: Amount::from_subunits(burned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(n: u8) -> UserId {
        UserId::from_byte(n)
    }

    #[test]
    fn one_to_two_split() {
        let plan = split_pot(
            Amount::from_subunits(90),
            &[
                (u(1), Amount::from_subunits(10)),
                (u(2), Amount::from_subunits(20)),
            ],
        )
        .unwrap();
        assert_eq!(plan.payouts[0].user, u(1));
        assert_eq!(plan.payouts[0].amount.subunits(), 30);
        assert_eq!(plan.payouts[1].amount.subunits(), 60);
        assert_eq!(plan.burned, Amount::ZERO);
    }

    #[test]
    fn remainder_is_burned() {
        let plan = split_pot(
            Amount::from_subunits(10),
            &[
                (u(1), Amount::from_subunits(1)),
                (u(2), Amount::from_subunits(1)),
            ],
        )
        .unwrap();
        assert_eq!(plan.payouts[0].amount.subunits(), 5);
        assert_eq!(plan.payouts[1].amount.subunits(), 5);
        assert_eq!(plan.burned, Amount::ZERO);

        let plan = split_pot(
            Amount::from_subunits(10),
            &[
                (u(1), Amount::from_subunits(1)),
                (u(2), Amount::from_subunits(2)),
            ],
        )
        .unwrap();
        // 10*1/3 = 3, 10*2/3 = 6, remainder 1 burned
        assert_eq!(plan.payouts[0].amount.subunits(), 3);
        assert_eq!(plan.payouts[1].amount.subunits(), 6);
        assert_eq!(plan.burned.subunits(), 1);
    }

    #[test]
    fn empty_winner_burns_entire_pot() {
        let plan = split_pot(Amount::TOKEN, &[]).unwrap();
        assert!(plan.payouts.is_empty());
        assert_eq!(plan.burned, Amount::TOKEN);
    }

    #[test]
    fn whale_gets_linear_share() {
        let plan = split_pot(
            Amount::from_tokens(10).unwrap(),
            &[
                (u(1), Amount::from_tokens(9).unwrap()),
                (u(2), Amount::TOKEN),
            ],
        )
        .unwrap();
        assert_eq!(plan.payouts[0].amount, Amount::from_tokens(9).unwrap());
        assert_eq!(plan.payouts[1].amount, Amount::TOKEN);
    }
}

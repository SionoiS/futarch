//! Derived balances: replay genesis + the settlement log. Not primary state.

use std::collections::BTreeMap;

use crate::amount::Amount;
use crate::error::Error;
use crate::genesis::Genesis;
use crate::settlement::SettlementEntry;
use crate::types::UserId;

/// Incremental cache of balances as of a log prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Balances {
    map: BTreeMap<UserId, Amount>,
    supply: Amount,
}

impl Balances {
    pub fn from_genesis(genesis: &Genesis) -> Self {
        let mut map = BTreeMap::new();
        let mut supply = Amount::ZERO;
        for f in genesis.founders() {
            map.insert(*f, Amount::TOKEN);
            supply = supply.saturating_add(Amount::TOKEN);
        }
        Self { map, supply }
    }

    pub fn get(&self, user: &UserId) -> Amount {
        self.map.get(user).copied().unwrap_or(Amount::ZERO)
    }

    pub fn supply(&self) -> Amount {
        self.supply
    }

    /// Apply one verified settlement. Forfeitures first, then pot shares, then free mints.
    pub fn apply_settlement(&mut self, entry: &SettlementEntry) -> Result<(), Error> {
        // Burn is implied by forfeitures not fully paid out. Check before mutating.
        if implied_burn(entry)? != entry.burned {
            return Err(Error::SettlementHashMismatch);
        }
        for f in &entry.forfeitures {
            let next = self.get(&f.user).checked_sub_err(f.amount)?;
            self.write(f.user, next);
            self.supply = self.supply.checked_sub_err(f.amount)?;
        }
        for p in &entry.payouts {
            let next = self.get(&p.user).checked_add_err(p.amount)?;
            self.write(p.user, next);
            self.supply = self.supply.checked_add_err(p.amount)?;
        }
        for user in &entry.free_mints {
            if self.get(user).is_zero() {
                self.write(*user, Amount::TOKEN);
                self.supply = self.supply.checked_add_err(Amount::TOKEN)?;
            }
        }
        Ok(())
    }

    fn write(&mut self, user: UserId, amount: Amount) {
        if amount.is_zero() {
            self.map.remove(&user);
        } else {
            self.map.insert(user, amount);
        }
    }
}

fn implied_burn(entry: &SettlementEntry) -> Result<Amount, Error> {
    let mut lost = Amount::ZERO;
    for f in &entry.forfeitures {
        lost = lost.checked_add_err(f.amount)?;
    }
    let mut paid = Amount::ZERO;
    for p in &entry.payouts {
        paid = paid.checked_add_err(p.amount)?;
    }
    lost.checked_sub_err(paid)
}

/// Replay genesis plus the full log into a fresh cache.
pub fn replay(genesis: &Genesis, log: &[SettlementEntry]) -> Result<Balances, Error> {
    let mut balances = Balances::from_genesis(genesis);
    for entry in log {
        balances.apply_settlement(entry)?;
    }
    Ok(balances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::RoomParams;
    use crate::settlement::{OutcomeKind, Payout, SettlementEntry};
    use crate::types::{Action, Hash, MessageId, ProposalId, Target};

    fn genesis_two() -> Genesis {
        Genesis::new(
            vec![UserId::from_byte(1), UserId::from_byte(2)],
            RoomParams::defaults(),
        )
        .unwrap()
    }

    #[test]
    fn founders_get_one_token() {
        let g = genesis_two();
        let b = Balances::from_genesis(&g);
        assert_eq!(b.get(&UserId::from_byte(1)), Amount::TOKEN);
        assert_eq!(b.get(&UserId::from_byte(2)), Amount::TOKEN);
        assert_eq!(b.get(&UserId::from_byte(3)), Amount::ZERO);
        assert_eq!(b.supply(), Amount::from_tokens(2).unwrap());
    }

    #[test]
    fn payouts_and_burns_update_supply() {
        let g = genesis_two();
        let mut b = Balances::from_genesis(&g);
        let loser = UserId::from_byte(1);
        let winner = UserId::from_byte(2);
        let entry = SettlementEntry::new(
            g.hash(),
            ProposalId::from_byte(1),
            Target::Message(MessageId::from_byte(1)),
            Action::RemoveMessage,
            OutcomeKind::NoDefault,
            vec![Payout {
                user: winner,
                amount: Amount::from_subunits(Amount::TOKEN.subunits() - 1),
            }],
            vec![Payout {
                user: loser,
                amount: Amount::TOKEN,
            }],
            vec![],
            Amount::from_subunits(1),
            None,
        );
        b.apply_settlement(&entry).unwrap();
        assert_eq!(b.get(&loser), Amount::ZERO);
        assert_eq!(
            b.get(&winner).subunits(),
            Amount::TOKEN.subunits() + Amount::TOKEN.subunits() - 1
        );
        assert_eq!(
            b.supply().subunits(),
            Amount::from_tokens(2).unwrap().subunits() - 1
        );
    }

    #[test]
    fn free_mint_is_set_not_add() {
        let g = genesis_two();
        let mut b = Balances::from_genesis(&g);
        let newbie = UserId::from_byte(9);
        let entry = SettlementEntry::new(
            Hash::from_byte(0),
            ProposalId::from_byte(1),
            Target::Message(MessageId::from_byte(1)),
            Action::RemoveMessage,
            OutcomeKind::YesExecuted,
            vec![],
            vec![],
            vec![newbie, newbie],
            Amount::ZERO,
            None,
        );
        b.apply_settlement(&entry).unwrap();
        assert_eq!(b.get(&newbie), Amount::TOKEN);
        assert_eq!(b.supply(), Amount::from_tokens(3).unwrap());
    }

    #[test]
    fn mismatched_burn_is_rejected() {
        let g = genesis_two();
        let mut b = Balances::from_genesis(&g);
        let entry = SettlementEntry::new(
            g.hash(),
            ProposalId::from_byte(1),
            Target::Message(MessageId::from_byte(1)),
            Action::RemoveMessage,
            OutcomeKind::NoDefault,
            vec![],
            vec![Payout {
                user: UserId::from_byte(1),
                amount: Amount::TOKEN,
            }],
            vec![],
            Amount::ZERO, // should be TOKEN
            None,
        );
        assert_eq!(
            b.apply_settlement(&entry).unwrap_err(),
            Error::SettlementHashMismatch
        );
    }
}

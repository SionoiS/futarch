//! Token amounts in subunits.
//!
//! One display token is [`TOKEN`] subunits (1_000_000). Genesis still grants
//! “1 token”; the finer unit makes √S weighting and pro-rata payouts meaningful.

use crate::error::Error;

/// Subunits in one display token.
pub const TOKEN: u64 = 1_000_000;

/// Non-negative token quantity, stored as subunits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(u64);

impl Amount {
    /// Zero subunits.
    pub const ZERO: Self = Self(0);

    /// Exactly one display token.
    pub const TOKEN: Self = Self(TOKEN);

    /// Construct from raw subunits.
    pub const fn from_subunits(n: u64) -> Self {
        Self(n)
    }

    /// Construct `n` display tokens, or `None` if the product overflows `u64`.
    pub const fn from_tokens(n: u64) -> Option<Self> {
        match n.checked_mul(TOKEN) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Raw subunit count.
    pub const fn subunits(self) -> u64 {
        self.0
    }

    /// `true` when the amount is zero.
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Checked addition.
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Checked subtraction.
    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Addition that errors instead of wrapping.
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Subtraction floored at zero.
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Addition used on consensus paths: overflow is an error.
    pub(crate) fn checked_add_err(self, other: Self) -> Result<Self, Error> {
        self.checked_add(other).ok_or(Error::Overflow)
    }

    /// Subtraction used on consensus paths.
    pub(crate) fn checked_sub_err(self, other: Self) -> Result<Self, Error> {
        self.checked_sub(other).ok_or(Error::InsufficientBalance)
    }
}

impl core::fmt::Display for Amount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} subunits", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_constant_is_one_million() {
        assert_eq!(TOKEN, 1_000_000);
        assert_eq!(Amount::TOKEN.subunits(), TOKEN);
    }

    #[test]
    fn from_tokens_multiplies() {
        assert_eq!(Amount::from_tokens(3).unwrap().subunits(), 3_000_000);
        assert!(Amount::from_tokens(u64::MAX).is_none());
    }

    #[test]
    fn checked_arithmetic() {
        let a = Amount::from_subunits(10);
        let b = Amount::from_subunits(4);
        assert_eq!(a.checked_add(b).unwrap().subunits(), 14);
        assert_eq!(a.checked_sub(b).unwrap().subunits(), 6);
        assert!(a.checked_sub(Amount::from_subunits(11)).is_none());
        assert!(
            Amount::from_subunits(u64::MAX)
                .checked_add(Amount::from_subunits(1))
                .is_none()
        );
    }

    #[test]
    fn saturating_floor_and_cap() {
        assert_eq!(
            Amount::from_subunits(3).saturating_sub(Amount::from_subunits(10)),
            Amount::ZERO
        );
        assert_eq!(
            Amount::from_subunits(u64::MAX).saturating_add(Amount::from_subunits(1)),
            Amount::from_subunits(u64::MAX)
        );
    }
}

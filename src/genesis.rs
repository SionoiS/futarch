//! Genesis record: founding members and frozen room parameters.

use crate::crypto::hash_tagged;
use crate::encode::{Encoder, TAG_GENESIS};
use crate::error::Error;
use crate::params::RoomParams;
use crate::types::{Hash, UserId};

/// Room birth record. Each founder is granted exactly one token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Genesis {
    founders: Vec<UserId>,
    params: RoomParams,
}

impl Genesis {
    /// Reject empty or duplicate founder lists.
    pub fn new(founders: Vec<UserId>, params: RoomParams) -> Result<Self, Error> {
        if founders.is_empty() {
            return Err(Error::GenesisEmpty);
        }
        let mut seen = std::collections::BTreeSet::new();
        for f in &founders {
            if !seen.insert(*f) {
                return Err(Error::DuplicateFounder);
            }
        }
        Ok(Self { founders, params })
    }

    pub fn founders(&self) -> &[UserId] {
        &self.founders
    }

    pub fn params(&self) -> &RoomParams {
        &self.params
    }

    pub fn hash(&self) -> Hash {
        hash_tagged(TAG_GENESIS, |e| {
            encode_params(e, &self.params);
            e.u32(self.founders.len() as u32);
            for f in &self.founders {
                e.bytes32(f.as_bytes());
            }
        })
    }
}

pub(crate) fn encode_params(e: &mut Encoder, p: &RoomParams) {
    encode_threshold(e, p.remove_message);
    encode_threshold(e, p.short_restrict);
    encode_threshold(e, p.medium_restrict);
    encode_threshold(e, p.long_restrict);
    encode_threshold(e, p.permanent_restrict);
    e.u64(p.short_max.secs());
    e.u64(p.medium_max.secs());
    e.u64(p.long_max.secs());
    e.u64(p.betting_window_ms);
    e.u64(p.commit_to_reveal_window_ms);
    e.u64(p.opening_window_ms);
}

fn encode_threshold(e: &mut Encoder, t: crate::params::SeverityThreshold) {
    e.u64(t.ratio.numerator);
    e.u64(t.ratio.denominator);
    e.u64(t.floor_bps);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::RoomParams;
    use crate::types::UserId;

    #[test]
    fn rejects_empty_and_duplicates() {
        assert_eq!(
            Genesis::new(vec![], RoomParams::defaults()).unwrap_err(),
            Error::GenesisEmpty
        );
        let a = UserId::from_byte(1);
        assert_eq!(
            Genesis::new(vec![a, a], RoomParams::defaults()).unwrap_err(),
            Error::DuplicateFounder
        );
    }

    #[test]
    fn hash_changes_with_founder_order() {
        let a = UserId::from_byte(1);
        let b = UserId::from_byte(2);
        let g1 = Genesis::new(vec![a, b], RoomParams::defaults()).unwrap();
        let g2 = Genesis::new(vec![b, a], RoomParams::defaults()).unwrap();
        assert_ne!(g1.hash(), g2.hash());
    }
}

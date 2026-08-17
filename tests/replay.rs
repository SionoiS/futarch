//! Live room vs `from_log` reconstruction.

mod common;

use common::{
    commit_free, commit_paid, default_room, open_remove, open_restrict, reveal_and_open, settle,
    t0, uid,
};
use futarch::{Amount, Direction, DurationSecs, Genesis, Room, Timestamp};

#[test]
fn from_log_matches_live_after_mixed_sequence() {
    let mut live = default_room([1, 2, 3]);
    let now = t0();

    let (id, o1) = open_remove(&mut live, uid(1), Direction::Yes, 1, now);
    let o2 = commit_paid(&mut live, uid(2), id, Direction::No, 2, Amount::TOKEN, now);
    let free = commit_free(&mut live, uid(9), id, Direction::No, 9, now);
    reveal_and_open(&mut live, uid(1), id, &o1, Amount::TOKEN, now);
    reveal_and_open(&mut live, uid(2), id, &o2, Amount::TOKEN, now);
    reveal_and_open(&mut live, uid(9), id, &free, Amount::ZERO, now);
    settle(&mut live, id, now);

    let now2 = Timestamp::from_millis(now.millis() + 50_000_000);
    let (id2, o3) = open_restrict(
        &mut live,
        uid(2),
        uid(8),
        DurationSecs::from_secs(120),
        Direction::Yes,
        3,
        now2,
    );
    reveal_and_open(&mut live, uid(2), id2, &o3, Amount::TOKEN, now2);
    settle(&mut live, id2, now2);

    let rebuilt = Room::from_log(
        Genesis::new(
            vec![uid(1), uid(2), uid(3)],
            futarch::RoomParams::defaults(),
        )
        .unwrap(),
        live.settlements().to_vec(),
    )
    .unwrap();

    for n in 1..=9u8 {
        let u = uid(n);
        assert_eq!(live.balance(&u), rebuilt.balance(&u), "balance {n}");
    }
    assert_eq!(live.supply(), rebuilt.supply());
    let t = common::at_settle(&live, now2);
    assert_eq!(
        live.effective_restriction(&uid(8), t),
        rebuilt.effective_restriction(&uid(8), t)
    );
    assert_eq!(rebuilt.open_proposals().count(), 0);
}

#[test]
fn tampered_settlement_rejected() {
    let mut live = default_room([1, 2]);
    let now = t0();
    let (id, o) = open_remove(&mut live, uid(1), Direction::Yes, 1, now);
    reveal_and_open(&mut live, uid(1), id, &o, Amount::TOKEN, now);
    settle(&mut live, id, now);

    let mut entries = live.settlements().to_vec();
    entries[0].burned = Amount::from_subunits(99);

    let err = Room::from_log(
        Genesis::new(vec![uid(1), uid(2)], futarch::RoomParams::defaults()).unwrap(),
        entries,
    )
    .unwrap_err();
    assert_eq!(err, futarch::Error::SettlementHashMismatch);
}

#[test]
fn broken_chain_rejected() {
    let mut live = default_room([1, 2]);
    let now = t0();
    let (id, o) = open_remove(&mut live, uid(1), Direction::Yes, 1, now);
    reveal_and_open(&mut live, uid(1), id, &o, Amount::TOKEN, now);
    settle(&mut live, id, now);

    let mut entries = live.settlements().to_vec();
    entries[0].prev_hash = futarch::Hash::from_byte(99);
    entries[0].this_hash = entries[0].compute_hash();

    let err = Room::from_log(
        Genesis::new(vec![uid(1), uid(2)], futarch::RoomParams::defaults()).unwrap(),
        entries,
    )
    .unwrap_err();
    assert_eq!(err, futarch::Error::BrokenHashChain);
}

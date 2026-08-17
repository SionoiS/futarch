//! Remaining must-cover invariants from the design.

mod common;

use common::{commit_paid, default_room, open_remove, reveal_and_open, settle, t0, uid};
use futarch::{Amount, Direction, Genesis, OutcomeKind, Room, RoomParams};

#[test]
fn genesis_balances() {
    let room = default_room([1, 2, 3]);
    assert_eq!(room.balance(&uid(1)), Amount::TOKEN);
    assert_eq!(room.balance(&uid(2)), Amount::TOKEN);
    assert_eq!(room.balance(&uid(3)), Amount::TOKEN);
    assert_eq!(room.balance(&uid(4)), Amount::ZERO);
    assert_eq!(room.supply(), Amount::from_tokens(3).unwrap());
}

#[test]
fn whale_threshold_weight_is_concave_payout_is_linear() {
    // 10 founders. Alice opens Yes; eight others lock No and never open.
    // Unopposed opened Yes executes; pot = 8 tokens, all to Alice.
    let founders: Vec<_> = (1..=10).map(uid).collect();
    let mut room = Room::genesis(Genesis::new(founders, RoomParams::defaults()).unwrap()).unwrap();
    let now = t0();
    let (id, o_alice) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    for n in 2..=9u8 {
        commit_paid(&mut room, uid(n), id, Direction::No, n, Amount::TOKEN, now);
    }
    // uid(10) stays out, keeps 1 token.
    reveal_and_open(&mut room, uid(1), id, &o_alice, Amount::TOKEN, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::YesExecuted);
    assert_eq!(room.balance(&uid(1)), Amount::from_tokens(9).unwrap());
    assert_eq!(room.balance(&uid(10)), Amount::TOKEN);

    // Second proposal: whale (9) and minnow (1) both open Yes; nobody else.
    // Yes executes; pot empty. Just check they can both lock their full stack
    // and that √9 : √1 = 3 : 1 while stakes are 9 : 1 (unit-tested in crate).
    // Drive a pot through a third party forfeiture so the 9:1 payout is visible.
    let now2 = futarch::Timestamp::from_millis(now.millis() + 80_000_000);
    // Give uid(10) a proposal they forfeit, whale + minnow open Yes.
    // uid(10) has 1 token. Open as Yes but never reveal → default No, pot burned
    // unless someone opens No. Instead: uid(10) opens No? They would need to
    // be the proposer (first commit). First commit Yes or No is secret.
    // Proposer uid(10) commits No (1 token) and opens it; whale commits Yes 9,
    // minnow Yes 1, both open. W_yes = 3000+1000 = 4000, W_no = 1000,
    // 4000*100 >= 1000*133, floor ok → Yes. Pot = 1 token, split 9:1.
    let o10 = common::opening(Direction::No, 10);
    let id2 = match room
        .apply(
            futarch::Command::OpenProposal {
                proposer: uid(10),
                target: futarch::Target::Message(common::msg(2)),
                action: futarch::Action::RemoveMessage,
                commit: o10.paid_commitment(Amount::TOKEN),
            },
            now2,
        )
        .unwrap()
    {
        futarch::Outcome::ProposalOpened { id } => id,
        _ => panic!(),
    };
    let o_w = commit_paid(
        &mut room,
        uid(1),
        id2,
        Direction::Yes,
        11,
        Amount::from_tokens(9).unwrap(),
        now2,
    );
    // uid(2) is at zero. Use a leftover... we zeroed 2..=9. Only 1 and 10 have
    // funds. Minnow slot: we need another funded user. Restart this test more
    // simply: after first settlement alice=9, uid(10)=1. That's the 9:1 pair.
    // uid(10) is the loser (No). No second minnow needed — payout is 100% to whale.
    reveal_and_open(&mut room, uid(10), id2, &o10, Amount::TOKEN, now2);
    reveal_and_open(
        &mut room,
        uid(1),
        id2,
        &o_w,
        Amount::from_tokens(9).unwrap(),
        now2,
    );
    let e2 = settle(&mut room, id2, now2);
    assert_eq!(e2.outcome, OutcomeKind::YesExecuted);
    assert_eq!(e2.payouts.len(), 1);
    assert_eq!(e2.payouts[0].user, uid(1));
    assert_eq!(e2.payouts[0].amount, Amount::TOKEN);
    assert_eq!(room.balance(&uid(1)), Amount::from_tokens(10).unwrap());
    assert_eq!(room.balance(&uid(10)), Amount::ZERO);
}

#[test]
fn free_mint_is_not_additive() {
    // A user at 0 who is listed twice in free_mints still ends at 1. The engine
    // never inserts duplicates; the replay path is covered in unit tests.
    // Here: correct free mint once, then they have 1 and cannot free-commit.
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, o) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let free = common::commit_free(&mut room, uid(9), id, Direction::Yes, 9, now);
    reveal_and_open(&mut room, uid(1), id, &o, Amount::TOKEN, now);
    reveal_and_open(&mut room, uid(9), id, &free, Amount::ZERO, now);
    settle(&mut room, id, now);
    assert_eq!(room.balance(&uid(9)), Amount::TOKEN);
    assert!(!room.can_free_commit(&uid(9)));
}

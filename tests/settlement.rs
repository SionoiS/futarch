//! Settlement, forfeiture, default-No, Yes execute, free mint.

mod common;

use common::{
    at_settle, commit_free, commit_paid, default_room, open_remove, reveal_and_open, settle, t0,
    uid,
};
use futarch::{Amount, Command, Direction, Error, Outcome, OutcomeKind, Timestamp};

#[test]
fn unopposed_opened_yes_executes() {
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id, o) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    reveal_and_open(&mut room, uid(1), id, &o, Amount::TOKEN, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::YesExecuted);
    assert_eq!(room.balance(&uid(1)), Amount::TOKEN);
    assert!(room.proposal(&id).is_none());
}

#[test]
fn unopened_lock_is_forfeited_and_no_wins_by_default() {
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::NoDefault);
    assert_eq!(entry.burned, Amount::TOKEN);
    assert_eq!(room.balance(&uid(1)), Amount::ZERO);
    assert_eq!(room.supply(), Amount::from_tokens(2).unwrap());
}

#[test]
fn settle_with_zero_openings_still_runs() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let _ = settle(&mut room, id, now);
    assert_eq!(room.settlements().len(), 1);
    assert_eq!(room.open_proposals().count(), 0);
}

#[test]
fn split_decision_equal_weights_defaults_to_no() {
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id, o1) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let o2 = commit_paid(&mut room, uid(2), id, Direction::No, 2, Amount::TOKEN, now);
    reveal_and_open(&mut room, uid(1), id, &o1, Amount::TOKEN, now);
    reveal_and_open(&mut room, uid(2), id, &o2, Amount::TOKEN, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::NoDefault);
    // Yes stake (alice) is forfeited into the pot; bob (No) receives it.
    assert_eq!(room.balance(&uid(1)), Amount::ZERO);
    assert_eq!(room.balance(&uid(2)), Amount::from_tokens(2).unwrap());
}

#[test]
fn two_yes_one_no_executes() {
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id, o1) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let o2 = commit_paid(&mut room, uid(2), id, Direction::No, 2, Amount::TOKEN, now);
    let o3 = commit_paid(&mut room, uid(3), id, Direction::Yes, 3, Amount::TOKEN, now);
    reveal_and_open(&mut room, uid(1), id, &o1, Amount::TOKEN, now);
    reveal_and_open(&mut room, uid(2), id, &o2, Amount::TOKEN, now);
    reveal_and_open(&mut room, uid(3), id, &o3, Amount::TOKEN, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::YesExecuted);
    // Pot is bob's 1 token, split 1:1 between alice and charlie.
    assert_eq!(entry.payouts.len(), 2);
    assert_eq!(
        entry.payouts[0].amount,
        Amount::from_subunits(Amount::TOKEN.subunits() / 2)
    );
    assert_eq!(room.balance(&uid(2)), Amount::ZERO);
    assert_eq!(
        room.balance(&uid(1)),
        Amount::from_subunits(Amount::TOKEN.subunits() + Amount::TOKEN.subunits() / 2)
    );
}

#[test]
fn free_commit_correct_mints_one_and_gets_no_pot() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let newbie = uid(9);
    let (id, o1) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let free = commit_free(&mut room, newbie, id, Direction::Yes, 9, now);
    reveal_and_open(&mut room, uid(1), id, &o1, Amount::TOKEN, now);
    reveal_and_open(&mut room, newbie, id, &free, Amount::ZERO, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::YesExecuted);
    assert_eq!(entry.free_mints, vec![newbie]);
    assert!(entry.payouts.iter().all(|p| p.user != newbie));
    assert_eq!(room.balance(&newbie), Amount::TOKEN);
    assert_eq!(room.supply(), Amount::from_tokens(3).unwrap());
}

#[test]
fn free_commit_wrong_side_mints_nothing() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let newbie = uid(9);
    let (id, o1) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let free = commit_free(&mut room, newbie, id, Direction::No, 9, now);
    reveal_and_open(&mut room, uid(1), id, &o1, Amount::TOKEN, now);
    reveal_and_open(&mut room, newbie, id, &free, Amount::ZERO, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::YesExecuted);
    assert!(entry.free_mints.is_empty());
    assert_eq!(room.balance(&newbie), Amount::ZERO);
}

#[test]
fn unopened_free_commit_mints_nothing() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, o1) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    commit_free(&mut room, uid(9), id, Direction::Yes, 9, now);
    reveal_and_open(&mut room, uid(1), id, &o1, Amount::TOKEN, now);
    let entry = settle(&mut room, id, now);
    assert!(entry.free_mints.is_empty());
    assert_eq!(room.balance(&uid(9)), Amount::ZERO);
}

#[test]
fn second_outstanding_free_commit_rejected() {
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id_a, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let id_b = {
        let o = common::opening(Direction::Yes, 2);
        match room
            .apply(
                Command::OpenProposal {
                    proposer: uid(2),
                    target: futarch::Target::Message(common::msg(2)),
                    action: futarch::Action::RemoveMessage,
                    commit: o.paid_commitment(Amount::TOKEN),
                },
                now,
            )
            .unwrap()
        {
            Outcome::ProposalOpened { id } => id,
            _ => panic!(),
        }
    };
    commit_free(&mut room, uid(9), id_a, Direction::Yes, 9, now);
    let err = room
        .apply(
            Command::CommitFree {
                user: uid(9),
                proposal: id_b,
                commit: common::opening(Direction::Yes, 10).free_commitment(),
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, Error::OutstandingFreeCommit);
}

#[test]
fn holder_cannot_free_commit() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(
            Command::CommitFree {
                user: uid(2),
                proposal: id,
                commit: common::opening(Direction::Yes, 2).free_commitment(),
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, Error::BalanceNotZero);
}

#[test]
fn user_returning_to_zero_may_free_commit_again() {
    let mut room = default_room([1, 2]);
    let now = t0();
    // Drive alice to zero via forfeiture.
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    settle(&mut room, id, now);
    assert_eq!(room.balance(&uid(1)), Amount::ZERO);
    assert!(room.can_free_commit(&uid(1)));
    assert!(!room.can_open(&uid(1)));

    let now2 = Timestamp::from_millis(now.millis() + 1);
    let (id2, o2) = open_remove(&mut room, uid(2), Direction::Yes, 2, now2);
    let free = commit_free(&mut room, uid(1), id2, Direction::Yes, 11, now2);
    reveal_and_open(&mut room, uid(2), id2, &o2, Amount::TOKEN, now2);
    reveal_and_open(&mut room, uid(1), id2, &free, Amount::ZERO, now2);
    settle(&mut room, id2, now2);
    assert_eq!(room.balance(&uid(1)), Amount::TOKEN);
}

#[test]
fn cannot_settle_before_opening_deadline() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(Command::Settle { proposal: id }, now)
        .unwrap_err();
    assert_eq!(err, Error::NotYetSettlable);
    let _ = at_settle(&room, now);
}

#[test]
fn empty_winning_side_burns_pot() {
    // Nobody opens No, Yes fails the floor/ratio so No wins with zero openers.
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let entry = settle(&mut room, id, now);
    assert_eq!(entry.outcome, OutcomeKind::NoDefault);
    assert!(entry.payouts.is_empty());
    assert_eq!(entry.burned, Amount::TOKEN);
}

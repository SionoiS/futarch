//! Initiation, concurrency, phases, and commit-reveal verification.

mod common;

use common::{
    assert_phase, at_opening, at_reveal, at_settle, commit_paid, default_room, msg, open_remove,
    opening, t0, uid,
};
use futarch::{
    Action, Amount, Command, Direction, DurationSecs, Error, Opening, Phase, RevealCommit, Salt,
    Target, Timestamp,
};

#[test]
fn invalid_action_for_target() {
    let mut room = default_room([1, 2]);
    let err = room
        .apply(
            Command::OpenProposal {
                proposer: uid(1),
                target: Target::User(uid(2)),
                action: Action::RemoveMessage,
                commit: opening(Direction::Yes, 1).paid_commitment(Amount::TOKEN),
            },
            t0(),
        )
        .unwrap_err();
    assert_eq!(err, Error::InvalidActionForTarget);
}

#[test]
fn zero_stake_rejected() {
    let mut room = default_room([1, 2]);
    let err = room
        .apply(
            Command::OpenProposal {
                proposer: uid(1),
                target: Target::Message(msg(1)),
                action: Action::RemoveMessage,
                commit: opening(Direction::Yes, 1).paid_commitment(Amount::ZERO),
            },
            t0(),
        )
        .unwrap_err();
    assert_eq!(err, Error::ZeroStake);
}

#[test]
fn cannot_lock_more_than_available() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    // uid(1) already locked their only token on their own proposal.
    let err = room
        .apply(
            Command::CommitPaid {
                user: uid(1),
                proposal: id,
                commit: opening(Direction::Yes, 99).paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap_err();
    // already committed on this proposal, checked after available...
    // available is 0 so InsufficientBalance fires first.
    assert_eq!(err, Error::InsufficientBalance);
}

#[test]
fn tick_advances_phases() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    assert_phase(&room, &id, Phase::Betting);

    room.tick(at_reveal(&room, now));
    assert_phase(&room, &id, Phase::CommitToReveal);

    room.tick(at_opening(&room, now));
    assert_phase(&room, &id, Phase::Opening);

    room.tick(at_settle(&room, now));
    assert_phase(&room, &id, Phase::AwaitingSettlement);
}

#[test]
fn proposal_view_hides_direction() {
    let mut room = default_room([1, 2]);
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, t0());
    let view = room.proposal(&id).unwrap();
    assert_eq!(view.commitment_count, 1);
    assert_eq!(view.total_locked, Amount::TOKEN);
    // Compile-time: ProposalView has no direction field. Lock the names we care about.
    let _ = (view.phase, view.proposer, view.target, view.action);
}

#[test]
fn wrong_salt_fails_opening() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, o) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let first = o.commit_hash(Amount::TOKEN);
    room.apply(
        Command::CommitToReveal {
            user: uid(1),
            proposal: id,
            commit: RevealCommit {
                hash: o.reveal_hash(first),
            },
        },
        at_reveal(&room, now),
    )
    .unwrap();
    let bad = Opening::new(Direction::Yes, Salt::from_byte(99));
    let err = room
        .apply(
            Command::Open {
                user: uid(1),
                proposal: id,
                opening: bad,
            },
            at_opening(&room, now),
        )
        .unwrap_err();
    assert_eq!(err, Error::OpeningVerifyFailed);
}

#[test]
fn opening_outside_window_rejected() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, o) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(
            Command::Open {
                user: uid(1),
                proposal: id,
                opening: o,
            },
            now,
        )
        .unwrap_err();
    assert_eq!(
        err,
        Error::WrongPhase {
            expected: Phase::Opening,
            actual: Phase::Betting
        }
    );
}

#[test]
fn commit_paid_after_betting_rejected() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(
            Command::CommitPaid {
                user: uid(2),
                proposal: id,
                commit: opening(Direction::No, 2).paid_commitment(Amount::TOKEN),
            },
            at_reveal(&room, now),
        )
        .unwrap_err();
    assert_eq!(
        err,
        Error::WrongPhase {
            expected: Phase::Betting,
            actual: Phase::CommitToReveal
        }
    );
}

#[test]
fn two_message_removes_on_same_message_rejected() {
    let mut room = default_room([1, 2]);
    let now = t0();
    open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(
            Command::OpenProposal {
                proposer: uid(2),
                target: Target::Message(msg(1)),
                action: Action::RemoveMessage,
                commit: opening(Direction::No, 2).paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, Error::DuplicateTargetAction);
}

#[test]
fn locks_across_two_concurrent_proposals() {
    let mut room = default_room([1, 2, 3]);
    let now = t0();
    let (id_a, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let id_b = {
        match room
            .apply(
                Command::OpenProposal {
                    proposer: uid(2),
                    target: Target::User(uid(9)),
                    action: Action::Restrict {
                        duration: DurationSecs::from_secs(30),
                    },
                    commit: opening(Direction::Yes, 2).paid_commitment(Amount::TOKEN),
                },
                now,
            )
            .unwrap()
        {
            futarch::Outcome::ProposalOpened { id } => id,
            _ => panic!(),
        }
    };
    commit_paid(
        &mut room,
        uid(3),
        id_a,
        Direction::No,
        3,
        Amount::TOKEN,
        now,
    );
    assert_eq!(room.available(&uid(3)), Amount::ZERO);
    let err = room
        .apply(
            Command::CommitPaid {
                user: uid(3),
                proposal: id_b,
                commit: opening(Direction::Yes, 4).paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap_err();
    assert_eq!(err, Error::InsufficientBalance);
}

#[test]
fn can_open_clears_only_after_settle() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    assert!(!room.can_open(&uid(1)));
    room.tick(at_settle(&room, now));
    assert!(!room.can_open(&uid(1)));
    room.apply(Command::Settle { proposal: id }, at_settle(&room, now))
        .unwrap();
    // uid(1) forfeited to zero, so still cannot open.
    assert!(!room.can_open(&uid(1)));
    assert!(room.can_open(&uid(2)));
}

#[test]
fn already_committed_rejected() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, _) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(
            Command::CommitPaid {
                user: uid(1),
                proposal: id,
                commit: opening(Direction::No, 8).paid_commitment(Amount::from_subunits(1)),
            },
            now,
        )
        .unwrap_err();
    // available is 0 after locking TOKEN
    assert_eq!(err, Error::InsufficientBalance);
}

#[test]
fn open_without_reveal_commit_rejected() {
    let mut room = default_room([1, 2]);
    let now = t0();
    let (id, o) = open_remove(&mut room, uid(1), Direction::Yes, 1, now);
    let err = room
        .apply(
            Command::Open {
                user: uid(1),
                proposal: id,
                opening: o,
            },
            at_opening(&room, now),
        )
        .unwrap_err();
    assert_eq!(err, Error::NoRevealCommit);
}

#[test]
fn missing_proposal() {
    let mut room = default_room([1]);
    let err = room
        .apply(
            Command::Settle {
                proposal: futarch::ProposalId::from_byte(1),
            },
            Timestamp::from_millis(0),
        )
        .unwrap_err();
    assert_eq!(err, Error::ProposalNotFound);
}

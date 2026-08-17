//! Shared helpers for integration tests (public API only).

#![allow(dead_code)]

use futarch::{
    Action, Amount, Command, Direction, DurationSecs, Genesis, MessageId, Opening, Outcome, Phase,
    ProposalId, RevealCommit, Room, RoomParams, Salt, Target, Timestamp, UserId,
};

pub fn uid(n: u8) -> UserId {
    UserId::from_byte(n)
}

pub fn msg(n: u8) -> MessageId {
    MessageId::from_byte(n)
}

pub fn opening(dir: Direction, salt: u8) -> Opening {
    Opening::new(dir, Salt::from_byte(salt))
}

pub fn default_room(founders: impl IntoIterator<Item = u8>) -> Room {
    let founders: Vec<UserId> = founders.into_iter().map(uid).collect();
    Room::genesis(Genesis::new(founders, RoomParams::defaults()).unwrap()).unwrap()
}

pub fn t0() -> Timestamp {
    Timestamp::from_millis(1_000_000)
}

pub fn at_reveal(room: &Room, opened_at: Timestamp) -> Timestamp {
    opened_at.saturating_add_millis(room.params().betting_window_ms)
}

pub fn at_opening(room: &Room, opened_at: Timestamp) -> Timestamp {
    at_reveal(room, opened_at).saturating_add_millis(room.params().commit_to_reveal_window_ms)
}

pub fn at_settle(room: &Room, opened_at: Timestamp) -> Timestamp {
    at_opening(room, opened_at).saturating_add_millis(room.params().opening_window_ms)
}

pub fn open_remove(
    room: &mut Room,
    proposer: UserId,
    dir: Direction,
    salt: u8,
    now: Timestamp,
) -> (ProposalId, Opening) {
    let o = opening(dir, salt);
    let outcome = room
        .apply(
            Command::OpenProposal {
                proposer,
                target: Target::Message(msg(1)),
                action: Action::RemoveMessage,
                commit: o.paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap();
    match outcome {
        Outcome::ProposalOpened { id } => (id, o),
        other => panic!("expected ProposalOpened, got {other:?}"),
    }
}

pub fn open_restrict(
    room: &mut Room,
    proposer: UserId,
    target: UserId,
    duration: DurationSecs,
    dir: Direction,
    salt: u8,
    now: Timestamp,
) -> (ProposalId, Opening) {
    let o = opening(dir, salt);
    let outcome = room
        .apply(
            Command::OpenProposal {
                proposer,
                target: Target::User(target),
                action: Action::Restrict { duration },
                commit: o.paid_commitment(Amount::TOKEN),
            },
            now,
        )
        .unwrap();
    match outcome {
        Outcome::ProposalOpened { id } => (id, o),
        other => panic!("expected ProposalOpened, got {other:?}"),
    }
}

pub fn commit_paid(
    room: &mut Room,
    user: UserId,
    proposal: ProposalId,
    dir: Direction,
    salt: u8,
    amount: Amount,
    now: Timestamp,
) -> Opening {
    let o = opening(dir, salt);
    room.apply(
        Command::CommitPaid {
            user,
            proposal,
            commit: o.paid_commitment(amount),
        },
        now,
    )
    .unwrap();
    o
}

pub fn commit_free(
    room: &mut Room,
    user: UserId,
    proposal: ProposalId,
    dir: Direction,
    salt: u8,
    now: Timestamp,
) -> Opening {
    let o = opening(dir, salt);
    room.apply(
        Command::CommitFree {
            user,
            proposal,
            commit: o.free_commitment(),
        },
        now,
    )
    .unwrap();
    o
}

pub fn reveal_and_open(
    room: &mut Room,
    user: UserId,
    proposal: ProposalId,
    opening: &Opening,
    amount: Amount,
    opened_at: Timestamp,
) {
    let first_hash = opening.commit_hash(amount);
    let reveal = RevealCommit {
        hash: opening.reveal_hash(first_hash),
    };
    room.apply(
        Command::CommitToReveal {
            user,
            proposal,
            commit: reveal,
        },
        at_reveal(room, opened_at),
    )
    .unwrap();
    room.apply(
        Command::Open {
            user,
            proposal,
            opening: *opening,
        },
        at_opening(room, opened_at),
    )
    .unwrap();
}

pub fn settle(
    room: &mut Room,
    proposal: ProposalId,
    opened_at: Timestamp,
) -> futarch::SettlementEntry {
    match room
        .apply(Command::Settle { proposal }, at_settle(room, opened_at))
        .unwrap()
    {
        Outcome::Settled(e) => e,
        other => panic!("expected Settled, got {other:?}"),
    }
}

pub fn run_yes_unopposed(room: &mut Room, proposer: UserId) -> futarch::SettlementEntry {
    let now = t0();
    let (id, o) = open_remove(room, proposer, Direction::Yes, proposer.as_bytes()[0], now);
    reveal_and_open(room, proposer, id, &o, Amount::TOKEN, now);
    settle(room, id, now)
}

pub fn assert_phase(room: &Room, id: &ProposalId, phase: Phase) {
    assert_eq!(room.proposal(id).unwrap().phase, phase);
}

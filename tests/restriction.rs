//! Restriction supersession and expiry.

mod common;

use common::{default_room, open_restrict, reveal_and_open, settle, t0, uid};
use futarch::{Amount, Direction, DurationSecs, OutcomeKind, Room, Timestamp};

#[test]
fn longer_later_overrides_and_restarts_clock() {
    let mut room = default_room([1, 2, 3]);
    let target = uid(9);
    let now = t0();
    let (id, o) = open_restrict(
        &mut room,
        uid(1),
        target,
        DurationSecs::from_secs(3_600),
        Direction::Yes,
        1,
        now,
    );
    reveal_and_open(&mut room, uid(1), id, &o, Amount::TOKEN, now);
    let e1 = settle(&mut room, id, now);
    assert_eq!(e1.outcome, OutcomeKind::YesExecuted);
    let short = room
        .effective_restriction(&target, at_exec(&room, now))
        .unwrap();
    assert_eq!(short.duration, DurationSecs::from_secs(3_600));

    let now2 = Timestamp::from_millis(at_exec(&room, now).millis() + 1);
    let (id2, o2) = open_restrict(
        &mut room,
        uid(2),
        target,
        DurationSecs::from_secs(7_200),
        Direction::Yes,
        2,
        now2,
    );
    reveal_and_open(&mut room, uid(2), id2, &o2, Amount::TOKEN, now2);
    settle(&mut room, id2, now2);
    let exec2 = at_exec(&room, now2);
    let long = room.effective_restriction(&target, exec2).unwrap();
    assert_eq!(long.duration, DurationSecs::from_secs(7_200));
    assert_eq!(long.start, exec2);
}

#[test]
fn shorter_yes_pays_but_does_not_shorten() {
    let mut room = default_room([1, 2, 3]);
    let target = uid(9);
    let now = t0();
    let (id, o) = open_restrict(
        &mut room,
        uid(1),
        target,
        DurationSecs::from_secs(3_600),
        Direction::Yes,
        1,
        now,
    );
    reveal_and_open(&mut room, uid(1), id, &o, Amount::TOKEN, now);
    settle(&mut room, id, now);

    let now2 = Timestamp::from_millis(at_exec(&room, now).millis() + 1);
    let (id2, o2) = open_restrict(
        &mut room,
        uid(2),
        target,
        DurationSecs::from_secs(60),
        Direction::Yes,
        2,
        now2,
    );
    reveal_and_open(&mut room, uid(2), id2, &o2, Amount::TOKEN, now2);
    let e2 = settle(&mut room, id2, now2);
    assert_eq!(e2.outcome, OutcomeKind::YesExecuted);
    assert!(e2.executed_restriction.is_none());
    let active = room
        .effective_restriction(&target, at_exec(&room, now2))
        .unwrap();
    assert_eq!(active.duration, DurationSecs::from_secs(3_600));
    assert_eq!(active.start, at_exec(&room, now));
}

#[test]
fn expiry_restores_send_rights() {
    let mut room = default_room([1, 2]);
    let target = uid(9);
    let now = t0();
    let (id, o) = open_restrict(
        &mut room,
        uid(1),
        target,
        DurationSecs::from_secs(1),
        Direction::Yes,
        1,
        now,
    );
    reveal_and_open(&mut room, uid(1), id, &o, Amount::TOKEN, now);
    settle(&mut room, id, now);
    let start = at_exec(&room, now);
    assert!(room.effective_restriction(&target, start).is_some());
    assert!(
        room.effective_restriction(&target, Timestamp::from_millis(start.millis() + 1_000))
            .is_none()
    );
}

#[test]
fn permanent_never_expires() {
    let mut room = default_room([1, 2]);
    let target = uid(9);
    let now = t0();
    let (id, o) = open_restrict(
        &mut room,
        uid(1),
        target,
        DurationSecs::PERMANENT,
        Direction::Yes,
        1,
        now,
    );
    reveal_and_open(&mut room, uid(1), id, &o, Amount::TOKEN, now);
    settle(&mut room, id, now);
    assert!(
        room.effective_restriction(&target, Timestamp::from_millis(u64::MAX))
            .is_some()
    );
}

fn at_exec(room: &Room, opened_at: futarch::Timestamp) -> futarch::Timestamp {
    common::at_settle(room, opened_at)
}

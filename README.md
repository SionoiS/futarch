# futarch

A deterministic, transport-agnostic library for **futarchy-based chat
moderation**: participants place secret directional bets on proposed actions
(remove a message, restrict a user for duration `D`). When the supporting side
clears a relative threshold and a floor, the action executes and the winning
side takes the pot.

This crate is **not** a chat application and **not** a network stack. A host
authenticates users, supplies a clock, gossips `Command`s, and applies the
recorded consensus (hide a message, refuse to relay from a restricted user).

See [docs/](docs/README.md) for the mechanism design.

## Embedder sketch

```rust
use futarch::{
    Action, Amount, Command, Direction, Genesis, MessageId, Opening, Room,
    RoomParams, Salt, Target, Timestamp, UserId,
};

# fn main() -> Result<(), futarch::Error> {
let alice = UserId::from_byte(1);
let bob = UserId::from_byte(2);
let mut room = Room::genesis(Genesis::new(
    vec![alice, bob],
    RoomParams::defaults(),
)?)?;

let opening = Opening::new(Direction::Yes, Salt::from_byte(7));
let commit = opening.paid_commitment(Amount::TOKEN);
let now = Timestamp::from_millis(1_000);

room.apply(
    Command::OpenProposal {
        proposer: alice,
        target: Target::Message(MessageId::from_byte(9)),
        action: Action::RemoveMessage,
        commit,
    },
    now,
)?;
# let _ = (bob, room);
# Ok(())
# }
```

The host must persist the settlement log (and, if it needs crash recovery,
ephemeral open-proposal state). `Room::from_log` rebuilds balances and
restrictions from genesis plus the log; in-flight proposals are not in the log.

## Non-goals

Amount-hiding, time-lock encryption, early-release actions, adaptive
thresholds, inflation, identity / Sybil resistance, and networking are
explicitly deferred. See the design docs.

## License

Licensed under either of Apache License, Version 2.0 or MIT license, at your
option.

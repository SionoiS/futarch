# Design documents

These files are the source of truth for the mechanism this crate implements.
The library does not rewrite them.

| Document | Role |
|---|---|
| [overview.md](overview.md) | Why: vote on values, bet on beliefs; points as moderation power |
| [requirements.md](requirements.md) | Hard constraints: decentralization, mobile-first, derived balances |
| [mechanism.md](mechanism.md) | Rules: initiation, concurrency, duration actions, thresholds, genesis |
| [technical.md](technical.md) | Commit–reveal baseline, two-phase reveal, settlement log, escrow |

Numeric defaults (ratio, floor, window lengths, token subunits) that the
documents left open are chosen in `src/params.rs` and `src/amount.rs` and
frozen per room at genesis.

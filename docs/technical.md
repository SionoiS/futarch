# Decentralized Futarchy-Based Chat Moderation — Technical Aspects

## Key Resources Gathered So Far

### Futarchy & Decision Markets
- Robin Hanson – “Shall We Vote on Values, But Bet on Beliefs?” (foundational paper)
- Vitalik Buterin – “An Introduction to Futarchy” (Ethereum blog, 2014) – practical DAO-oriented sketch
- MetaDAO (Solana) – live conditional pass/fail markets using TWAP comparison and conditional vaults
- Futarchy Labs – infrastructure and documentation for decision / conditional / impact markets

### Secret Betting & Privacy-Preserving Techniques
**Lightweight / Mobile-friendly baseline**
- Commit-reveal schemes (hash commitment of direction + amount + salt; later reveal). Simple, fast on mobile, already defeats most front-running and copy-trading during the betting window.

**Stronger privacy options (evaluate carefully against mobile constraint)**
- Zero-knowledge proofs (zkSNARKs, Bulletproofs, etc.) for proving bet validity without revealing details
- Fully Homomorphic Encryption (Zama fhEVM / Fhenix-style) – compute aggregates on encrypted data
- Threshold encryption + distributed key generation (ElGamal threshold, t-of-n decryption of aggregates only after betting closes)
- Pedersen commitments + deferred aggregate revelation
- MPC / garbled circuits for private aggregation

Notable prototypes:
- Anonymous / “dark” prediction markets using ElGamal threshold encryption + ZK proofs + Poseidon commitments
- BlindBet and similar FHE-based confidential prediction markets
- Various sealed-bid auction constructions using HE + ZKP + commitments

### Supporting Patterns
- Conditional vaults that can revert losing-side trades
- Time-weighted average price (TWAP) or similar for more robust threshold decisions
- Parimutuel or bonded reward distribution
- Optimistic execution + challenge periods (useful complement to pure thresholds)

---

## Commit-Reveal Baseline — Design Decisions (Aug 2026)

This section records the concrete rules chosen for the lightweight commit-reveal scheme. Amount-hiding is explicitly deferred to a future version.

### Secrecy properties (current baseline)
- **Direction is secret** during the entire betting window. Only a hash commitment is stored in ephemeral state; the pre-image (direction + amount + salt) remains on the user’s device until reveal.
- **Amount is public**. The locked quantity is visible in the ephemeral proposal state. This is an accepted trade-off for simplicity and mobile performance. Public amounts can still influence behaviour (intensity signalling, perceived seriousness, reputation inference), but the most damaging information for front-running and copy-trading — the side — remains hidden.
- **Future version**: value-hiding commitments (Pedersen or equivalent) or aggregate-only schemes so that both direction *and* amount stay secret. Not part of the current baseline.

### State model
**Persistent state** (lives for the lifetime of the chat/room):
- Chat content + participant list (integrity of ordinary messages is the responsibility of the underlying chat protocol)
- Append-only, hash-chained **settlement log** (and executed moderation actions)
- Genesis record (founding members and initial 1-token grants)

**Balances are derived state.**  
There is no mutable balances map stored as primary state. Current balances and total supply are recomputed by replaying the settlement log from genesis. Clients normally keep a local cache of “balances as of log position N” and apply only newer deltas.

**Ephemeral state** (lives only while a proposal is open + short cleanup window):
- Proposal metadata
- Commitment hashes
- Locked amounts associated with those hashes
- Betting / reveal timers

Ephemeral state is aggressively deleted at settlement. This satisfies the minimal-state requirement while still allowing real capital locks and clean forfeiture.

### Settlement log entry (the only durable economic record)
Each settlement produces one append-only log entry that contains at least:
- Proposal identifier and cryptographic reference to the target (message hash or user id)
- Outcome (yes executed / no default)
- List of successful openers on the winning side and the exact amount each receives from the pot
- Any free-commitment mint that occurred (`set balance = 1` for a user whose balance was still 0)
- Hash of the previous settlement entry (linear hash chain for integrity and ordering)

Replaying the chain from genesis yields the exact current balance of every participant and the total supply. Ordering of concurrent settlements is defined by their position in this chain.

### Escrow mechanics
Because tokens are room-local, the “escrow” is not a separate long-lived contract. It is a temporary accounting entry inside the ephemeral proposal state:

```
available[user]  -= amount
locked[commitmentHash] = amount
```

(The `available` values used at lock time are the balances derived from the settlement log at the moment the commitment is accepted.)

On settlement the locked amounts are reassigned according to the rules below and the entire ephemeral structure is discarded. No persistent escrow remains.

Free (+1) commitments create no lock at all.

### Non-reveal / forfeiture rule
- After the final opening deadline a settlement step **always** runs (permissionless or keeper-triggered), even if zero commitments were successfully opened.
- Only successfully opened and verified commitments count toward the threshold and toward reward distribution.
- Any still-unopened locked stake is **forfeited** and added to the reward pot.
- **Winning side determination**:
  - If yes clears both the relative threshold **and** the floor → action executes, yes is the winning side.
  - Otherwise → action does **not** execute, no is the winning side (default).
- The pot (losing-side opened stakes + all forfeited stakes) is distributed to the winning side’s successful openers **pro-rata according to the full stake each of them committed** (not according to the concave effective weight used for the threshold).
- Free commitments that are never opened simply have no effect and mint nothing. A free commitment is “correct” only if its direction matches the eventual winning side.
- After settlement the ephemeral proposal state is deleted. The only durable record is the new settlement log entry described above (which encodes the net balance deltas and, if applicable, the executed moderation action).

This guarantees progress, prevents stuck funds, and makes strategic non-revelation costly. Both sides always have real economic skin.

### Reveal-phase design (two-phase / nested commit-reveal) — chosen baseline
To eliminate sequential last-mover advantage on public reveals, the reveal process itself is split into two phases:

1. **Commit-to-reveal window** (starts when the betting window closes):  
   Users who wish to participate post a second commitment — a hash of the opening (direction + salt, bound to the original commitment).  
   No direction is made public during this window. The second commitment only proves the user still intends to open.

2. **Opening window** (short, fixed duration after the commit-to-reveal window):  
   Users open the second commitment by publishing the pre-image.  
   All successful openings are collected. Intermediate tallies of directions are not used for decisions; settlement occurs only after the opening window ends.

3. **Settlement** runs once after the opening deadline, using only the fully opened set.

This structure forces essentially simultaneous visibility of directions. A player cannot observe a running public tally of others’ directions and then decide whether to open a decisive commitment. The second (opening) window can be kept short, further reducing any residual timing games.

**Future work (not part of baseline):** Beacon-based time-lock encryption (e.g. tlock/drand) or pure computational time-lock puzzles of the openings. These can provide true simultaneous decryption without a second interactive window and are recorded as a possible later upgrade. Amount-hiding remains a separate future item.

### Summary of the flow
1. Betting window: users lock amount + post hash. Direction stays secret; amount is visible.
2. Betting window closes → commit-to-reveal window opens. Users post a second hash (commitment to the opening).
3. Commit-to-reveal window closes → short opening window. Users publish the pre-image of the second commitment.
4. Opening deadline → settlement always executes:
   - Tally successfully opened stakes (apply quadratic weighting if active).
   - Decide whether yes cleared threshold + floor.
   - If yes → execute action; yes wins the pot.
   - If no → do not execute; no wins the pot.
   - Forfeit unopened locks into the pot.
   - Compute payouts (pro-rata full stake on the winning side) and any free-commitment mint (`set balance = 1` if still at 0).
   - Append a single settlement log entry (hash-chained) that records the outcome, payouts, free mint, and target reference.
   - Delete all ephemeral proposal data.
   - Balances are thereafter obtained by replaying the settlement log.

---

## Client / UI Expectations

Because multiple proposals may be open concurrently, clients are expected to help focus user attention rather than relying on protocol-level serialization.

- Surface the highest-staked or highest-severity open proposals first (sortable list or prioritized view).
- Clearly distinguish proposal state (betting window vs commit-to-reveal window vs opening window vs settled).
- Make the target (message or user) and action type immediately visible so users can decide where to place attention and stake.

These are client responsibilities; the protocol itself only enforces the concurrency rules defined in `mechanism.md`.

---

## Open Design Questions / Missing Pieces

- Exact numeric default parameters for the threshold (the 1.33× margin, the size of the floor, and the concrete ratios / duration scaling). Direction is settled: relative yes-vs-no comparison plus floor, differentiated by severity; system ships with good defaults and the chat creator may override them once at room creation; adaptive moving-average remains a future option only.
- (Closed) Reward formula: pot is distributed pro-rata according to the full stake each winning-side successful revealer committed. The concave effective weight is used only for the threshold comparison, never for reward shares.
- Whether the system uses pure stake-threshold execution or true conditional futarchy markets on a success metric
- Resolution / oracle needs if rewards depend on “correctness” beyond the threshold itself
- Clean integration between the on-chain (or decentralized) betting layer and the actual chat application (off-chain action execution)
- Choice of cryptographic stack that satisfies both secrecy and strict mobile performance constraints (amount-hiding schemes remain future work)
- Long-lived chat refinements (optional mild continuous emission, adaptive threshold calibration, or other anti-ossification measures) once the core bootstrap is in place
- Explicit redeeming / early-release / rehabilitation actions (currently left as future work; core design only supports non-decreasing restrictions until natural expiry)
- (Closed) Last-mover advantage on reveals: solved in the baseline by two-phase / nested commit-reveal (commit-to-reveal window followed by a short opening window). Directions become visible essentially simultaneously; sequential observation of a running tally is eliminated. Beacon-based time-lock encryption (tlock/drand) and pure computational time-lock puzzles of openings are left as future work.

---

*Technical research, options, and open questions for implementing the Futarchy-based chat moderation system.*

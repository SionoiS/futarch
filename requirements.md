# Decentralized Futarchy-Based Chat Moderation — Requirements, Assumptions & Design Constraints

## Core Requirements

- **Fully decentralized.** No central server. The system must operate without any single trusted operator, coordinator, or always-on backend that can censor, alter state, or become a point of failure.
- **Mobile-first usability.** The mechanism must remain practical on ordinary mobile phones. Cryptographic choices, state size, computation, and interaction patterns are constrained by this requirement. Heavy schemes that make everyday participation painful on a phone are unacceptable for the baseline.

These two requirements dominate design trade-offs. Everything else is subordinate to them.

---

## Core Assumptions

- **Sybil resistance and user verification are out of scope.**  
  The design does not attempt to solve identity, unique-person guarantees, or proof-of-personhood. Tokens, balances, and voting power are room-local and earned through correct prediction inside that room. External identity systems, KYC, or global reputation are not required and are not assumed.

- Individual chats / rooms are treated as finite-lived by default (they are born, run for a period, then die; new rooms start from a fresh genesis). Under this assumption continuous inflation is not required for whale dilution; natural room death already resets power concentrations.

- “Good chat” is defined by the community itself rather than by fixed absolute rules. The mechanism continuously discovers and enforces evolving community standards via prediction markets.

- The underlying chat protocol already provides ordinary message integrity and delivery. The moderation layer only records consensus decisions and token flows. A full causal DAG of messages is not required by the core mechanism.

- Clients (not the protocol) are responsible for focusing user attention when multiple proposals are open concurrently.

---

## Explicit Requirements

### Mechanism
- A fully decentralized moderation gadget that lets chat participants place secret token bets on proposed moderation actions.
- An action executes only when the supporting side clears both a relative threshold and a protective floor. In all other cases the action does **not** execute and the opposing side wins the pot (**no side wins by default**).
- Successful revealers on the winning side receive the pot (opposing opened stakes + all forfeited unopened stakes) distributed pro-rata according to the **full stake** each of them committed.
- The system must be open-source.

### State model
- **Balances are derived state.** No mutable balances map is stored as primary state. Current balances and total supply are obtained by replaying an append-only, hash-chained settlement log from genesis.
- Persistent state consists of: chat content + participant list, the settlement log, and the genesis record.
- Ephemeral proposal state (commitments, locks, timers) is deleted on settlement. Only the new settlement-log entry remains.
- Ordinary chat-message integrity is delegated to the underlying chat protocol. The moderation gadget requires only the linear settlement chain + signed target references.

### Betting & reveal
- Direction is secret during the betting window (hash + salt). Amount is public in the current baseline (amount-hiding is deferred).
- Room-local ephemeral escrow: available → locked movement inside proposal state. Free commitments create no lock.
- Settlement always runs after the final opening deadline. Unopened locked stakes are forfeited into the pot.
- Last-mover advantage on reveals is solved by two-phase / nested commit-reveal: after betting closes there is a commit-to-reveal window followed by a short opening window. Directions become visible essentially simultaneously.
- Free commitments that remain unopened are simply ignored and mint nothing.

### Token distribution & initiation
- Genesis: founding members each receive 1 token.
- Zero-balance users may submit free directional commitments on already-open proposals only (they cannot initiate).
- Free-commitment anti-farming rules:
  - At most one outstanding free commitment per user while balance == 0.
  - On correct settlement, if balance is still exactly 0, set balance = 1 (not additive +1).
  - No further free mints once balance ≥ 1.
  - Free committers never share the pot.
- Only users with balance > 0 (and no currently open proposal of their own) may open a new proposal, by posting the first real (paid) commitment.

### Proposal concurrency & action model
- **One open proposal per proposer** (primary anti-spam rule). Multiple proposals may still run concurrently if opened by different users.
- Exact duplicates are forbidden: at most one open proposal for any given `(target, action)` pair.
- Messages: at most one open proposal per message; the only action is remove.
- User actions are purely duration-based: the only action is “restrict sending messages for duration D”. Timeouts, temp bans, mutes, etc. are the same thing with different D. Permanent = D = ∞. Severity = length of D.
- Different durations on the same user may be proposed concurrently (subject to the one-proposal-per-proposer limit).
- **Supersession**: a later higher-severity (longer D) that passes overrides an active shorter restriction. The new duration starts from the moment the new proposal executes. Shorter proposals do not reduce an already-active longer restriction. Effective restriction is therefore non-decreasing until natural expiry.
- Early release / de-escalation via ordinary shorter proposals is not possible; it requires natural expiry. Explicit redeeming / early-release / rehabilitation actions are future work.

### Influence weighting
- Full stake S is always at risk and is used for all reward / pot calculations.
- Only the effective weight that contributes toward the execution threshold uses a concave function of stake (e.g. √S).
- The concave weight is never used for reward shares.

### Threshold
- Fixed absolute thresholds are rejected (supply varies by room).
- Relative rule: W_yes ≥ 1.33 × W_no plus a modest absolute or supply-relative floor.
- Thresholds are differentiated by severity (message removal vs short / medium / long / permanent duration).
- System ships with good defaults; the chat creator may override the parameters once at room creation.
- Threshold evaluation uses only post-reveal data + existing persistent state (total supply, balances, action log). No external oracles or real-time toxicity scores are required.

---

## Design Constraints

### Minimal state & mobile-friendliness
- Persistent state must remain minimal. Ephemeral proposal data is aggressively deleted after settlement.
- Baseline cryptography is commit-reveal (direction secret, amount public). Stronger privacy techniques (ZK, FHE, threshold encryption, amount-hiding commitments) are evaluated against the mobile constraint and treated as future options.
- Computation of effective weights, thresholds, and pro-rata payouts must remain trivial on ordinary mobile devices.

### Incentive & governance constraints
- Initiation power is reserved for users already selected by correct prediction (balance > 0). Zero-balance users are pure reactors.
- No extra parameters or opening fees beyond the binary “balance > 0” rule for initiation.
- Skin in the game is mandatory: every real commitment locks stake; unopened stakes are forfeited.
- Deep, persistent preference divergence is treated as schism, solved by clean exit and creation of a new room with fresh genesis distribution. Tokens are room-local and do not automatically carry over.
- Continuous inflation / mild emission and adaptive moving-average thresholds are retained only as optional future mechanisms for longer-lived communities; they are not part of the core design under the ephemeral-room assumption.

### Explicitly deferred / future work
- Amount-hiding (Pedersen or equivalent; both direction and amount secret).
- Beacon-based time-lock encryption (tlock / drand) or pure computational time-locks of openings.
- Explicit redeeming / early-release / rehabilitation actions.
- Adaptive moving-average of historical yes/no ratios.
- Mild continuous emission for anti-ossification in long-lived rooms.
- True conditional futarchy markets on a success metric (chat health, retention, etc.) instead of pure stake-threshold execution.
- Exact numeric default values for the 1.33× margin, floors, and duration scaling (direction is settled; concrete numbers still to be chosen).

---

*Requirements, assumptions, and design constraints for the Futarchy-based chat moderation system.  
See `overview.md` for the what & why, `mechanism.md` for the detailed rules, and `technical.md` for resources and open questions.*

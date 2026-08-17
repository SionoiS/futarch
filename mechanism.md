# Decentralized Futarchy-Based Chat Moderation — Mechanism Design

## Core Mechanism (as currently envisioned)

1. A moderation proposal is opened by a token holder for a specific action on a chat message or user (see Proposal Initiation below).
2. Users place **secret bets** (tokens) supporting or opposing the action.
3. If the supporting side meets the execution threshold (defined below), the action is executed.
4. Betters who correctly anticipated the outcome (or whose side prevailed) are rewarded from the opposing stakes / pot.
5. All of this happens in a decentralized, open-source manner with minimal trusted parties.

True futarchy flavor would involve *conditional markets* that forecast the impact of the moderation action on a chosen success metric (chat health, retention, toxicity, engagement, etc.). A pure stake-threshold version is simpler and closer to bonded / conviction-style proposals.

---

## Proposal Initiation

### Rule
Only a user whose current token balance is **strictly greater than zero** **and** who does not already have an open proposal may open a new proposal.  
Opening is performed by posting the first real (paid) commitment on a chosen target and action. That commitment creates the ephemeral proposal state and starts the betting + reveal clocks.

Zero-balance users **cannot** open proposals. They may only submit free directional commitments on proposals that are already open. The one-proposal-per-proposer limit also applies to them once they regain a balance > 0.

### Why this rule

- Holding tokens is already a (noisy but real) signal that the user has previously predicted community norms correctly. Initiation power is therefore reserved for people the mechanism has already selected.
- Zero-balance users are not locked out of the system; the existing free-commitment privilege still lets them earn their first (or next) token by reacting correctly to proposals opened by others. This preserves the skill-gated onboarding path.
- Allowing zero-balance users to open proposals would turn the free-commitment privilege into a spam vector and would give initiation rights to participants who have not yet demonstrated predictive skill.
- Requiring a minimum stake size *X* or a separate opening fee would introduce an extra parameter that must be set and possibly governed. The simple binary “balance > 0” needs no parameters and no extra state.
- Because the first commitment itself creates the proposal, every open proposal carries real skin in the game from the first moment. No separate creation privilege or fee machinery is required.
- Users who later return to a zero balance automatically lose the ability to open new proposals. This is intentional selection pressure: chronic mis-predictors lose initiation power until they prove themselves again via free commitments.

---

## Proposal Concurrency

### Core rules
- **One open proposal per proposer**: A user may have at most one open proposal at a time. While that proposal is still in its betting or reveal window, the same user cannot open another. This is the primary anti-spam / anti-dust rule.
- Multiple proposals may still be open concurrently in a room (opened by different users).
- Exact duplicates are forbidden: at most one open proposal is allowed for any given `(target, action-type)` pair.

### Messages
A message supports only a single meaningful action: **remove**.  
Therefore at most one open proposal is allowed per message. Once the proposal settles (executed or rejected), the slot is free, but a successful removal ends the matter in practice.

### Users — Action model
All user-targeted moderation collapses to a single underlying action:

> Restrict this user from sending messages for duration **D**.

Timeouts, temporary bans, mutes, etc. are not distinct action types; they are simply different values of D.  
Permanent restriction is D = ∞ (or an explicit permanent flag).  
Severity is therefore totally ordered by duration: longer D is strictly more severe.

Because this is not a centralized server with hard technical enforcement, the recorded action represents **community consensus**. Clients and the chat layer may choose to respect it (filter messages, show warnings, refuse relay, etc.). The protocol itself only records and settles the consensus decision and the associated token flows.

### Users — Concurrency and uniqueness
- At most one open proposal is allowed per exact `(user, duration)` pair (or `(user, permanent)`).
- Different durations on the same user **are** permitted concurrently, subject to the one-proposal-per-proposer limit.
- Temporary restrictions expire on their own schedule. After expiry (or a short cooldown) new proposals on that user may be opened again.

### Users — Supersession rule (later higher severity overrides)
When a proposal for duration D₂ passes:

- If the target user currently has an active restriction of duration D₁ and D₂ > D₁, the new restriction **overrides**. The effective restriction becomes D₂, measured from the moment the new proposal executes.
- A proposal for a shorter duration does **not** reduce an already-active longer restriction. It may still clear its threshold and pay its bettors, but it has no effect on the active restriction.
- This makes the effective restriction non-decreasing until natural expiry (or until an explicit reversal mechanism is used).

**De-escalation / early release / redeeming actions**  
Under the pure “later higher overrides” rule, ordinary shorter-duration proposals cannot end or shorten an existing restriction. Early release therefore depends on natural expiry of the current restriction.

**Future work:** Explicit redeeming / early-release actions (e.g. “end restriction now” / set D = 0, or other rehabilitation proposals) are deliberately left out of the core design. They will require their own threshold rules, interaction with the supersession logic, and careful incentive analysis. This is recorded as future work to be done once the basic duration + later-higher-override mechanism has been exercised.

### Why this design

**One proposal per proposer**  
Prevents a single token holder from opening many concurrent dust or low-effort proposals to dilute attention, lock capital across many markets, or grief the room. A user who wants to act on multiple targets must wait for their current proposal to settle (or be rejected) before opening the next. This is simple to enforce (one flag or counter per user) and needs no extra parameters.

**Rejected: global single-proposal lock (room-wide)**  
A room-wide “only one proposal at a time” rule creates a sequential bottleneck. Chats are bursty; multiple independent problems can appear in seconds. While one proposal is open, other damage continues. It also enables cheap queue-blocking grief and reduces free-commitment opportunities for zero-balance users. Attention focusing is better solved by UI (sorting open proposals by locked stake, severity, or recency) than by a hard global lock. Different proposers may therefore run proposals in parallel.

**Simplified severity + later-higher-override**  
Because every user action is just a duration, severity ordering is objective (longer = more severe). Allowing multiple durations to be proposed concurrently preserves market expressiveness; the supersession rule then resolves conflicts at execution time without requiring bettors to be restricted. The non-decreasing nature of restrictions until expiry is accepted as a deliberate bias toward caution once the community has already imposed a longer sanction.

Deep unresolved preference divergence continues to be handled by the existing schism / exit path. Attention focusing under concurrency is left to client UI (see technical.md).

---

## Threshold for Execution

### Why a fixed absolute number is unsuitable
Token supply is room-local and variable. Rooms begin with different numbers of founding members and grow only through successful free commitments. A hard-coded token count is therefore either trivially easy or impossibly high depending on the room’s age and size. Any viable threshold must scale with the room.

### Preferred direction: relative comparison + floor (No wins by default)
The leading candidate is a **relative rule** that compares the two sides after reveal, combined with a protective floor. **The no side wins by default.**

- Let \(W_{\text{yes}}\) and \(W_{\text{no}}\) be the effective weights of the revealed supporting and opposing sides (raw stake or quadratic \(\sqrt{S}\), consistent with the influence-weighting choice).
- The action is executed **if and only if**  
  \(W_{\text{yes}} \ge 1.33 \times W_{\text{no}}\)  
  **and**  
  \(W_{\text{yes}}\) clears a modest floor (absolute or a small percentage of total room supply).
- In all other cases the proposal fails, the action is **not** executed, and the no side is the winner.

The 33 % margin requires the supporting side to be clearly ahead rather than merely larger. The floor prevents near-zero-stake executions that would otherwise pass in low-attention situations or early in a room’s life. Both quantities are known at settlement time and require no extra persistent state beyond what is already present.

This default-to-no rule makes incentives symmetric: both sides put real stake at risk, and the market only overrides the status quo when yes is decisively ahead.

### Differentiation by severity (duration) and creator configuration
Different actions should not share the same bar. Because user actions are ordered purely by duration D, thresholds can scale with D (and with the binary message-removal case):

- Message removal — lowest ratio / lowest floor  
- Short durations — lower values  
- Medium durations — intermediate values  
- Long / permanent durations — substantially higher ratio and floor  

The system ships with **good defaults** for the ratio, floor, and any duration scaling.  
The chat creator may override these parameters at room creation (and only at creation, to avoid mid-stream manipulation).  

Adaptive moving-average threshold adjustment remains future work only (see below). Exact default numbers are still to be chosen; the design commitment is that longer (more severe) restrictions face a meaningfully higher bar and that creators have a one-time configuration right.

### Information available for threshold calculation
At settlement the following inputs are reliably present and cheap:

- Total token supply and the full balance vector, obtained by replaying the settlement log (enables supply-relative floors and quadratic transforms)
- Revealed yes and no effective weights
- Number of distinct revealers on each side
- Proposal timing (betting + reveal window lengths)
- Append-only history of previously executed actions / settlements (room age, past success counts)

Real-time online counts, message-level toxicity scores, or external “chat health” oracles are *not* assumed; they would violate the minimal-state and mobile-friendly constraints or require additional infrastructure.

### Adaptive moving-average threshold (future option)
A more elaborate scheme was examined: maintain a smoothed moving average of the realized yes/no ratios of *passed* proposals and slowly drift the required threshold toward that average, with separate averages per action type.  

This can discover room-specific norms and is attractive for long-lived communities. It is **not** part of the core design for the same reasons continuous inflation is optional:

- Cold-start problems in short-lived / ephemeral rooms  
- Path dependence and early-founder capture of the average  
- Survivorship bias (only successful proposals update the statistic)  
- Additional persistent state per action type  
- Manipulation surface via sequences of easy proposals  

It is retained as a possible future mechanism if the system is later extended to support longer-lived rooms that benefit from adaptive local calibration.

---

## Token Distribution & Bootstrapping

### Current mechanism

**Genesis**  
At chat initialization the creator lists the founding members. Each founding member receives **1 token**.

**Ongoing bootstrap for zero-balance users**  
While a user’s balance is exactly 0 they may submit free directional commitments (support or oppose) on already-open moderation proposals. They cannot open new proposals themselves.  

Rules to prevent farming:
- A user may have at most one outstanding (unsettled) free commitment at a time.
- On settlement, if the free commitment’s direction was correct **and** the user’s balance is still exactly 0, the protocol **sets the user’s balance to 1**.  
- No further free mints are possible once balance ≥ 1. Even if multiple free commitments were somehow pending, only the first successful settlement while still at 0 has effect; subsequent ones mint nothing.
- Free predictors do **not** receive any share of the opposing stakes / pot.
- Once the user holds ≥ 1 token, all subsequent bets require real stake and follow normal reward rules; they also regain the ability to open proposals.
- If a user later returns to a zero balance through losses, the free-commitment privilege reactivates (soft safety net that still requires correctness) and the ability to open proposals is again suspended.

### Rationale

- Primary token acquisition remains correct prediction of community norms, preserving the core selection pressure of the system.  
- The free token is itself earned by a correct bet, not by mere presence or admin discretion after genesis. The effect is a hard “set to 1”, not an additive +1 that could be repeated.  
- Farming multiple tokens via free commitments is explicitly disallowed: at most one outstanding free commitment, and minting is capped at reaching balance = 1.  
- Presence-farming and pure time-based emission are avoided.  
- Works cleanly for ephemeral channels: the people who correctly called the (few) moderation events of a short-lived room receive the stake.  
- Provides a minimal, skill-gated on-ramp for newcomers and a soft recovery path for users who have been driven to zero, without permanently locking them out.  
- Zero-balance users remain pure reactors (they cannot open proposals); initiation power stays with those who already hold tokens.  
- State impact is negligible (balance check + at most one pending free-commitment flag per user).  
- Remains fully compatible with mobile-friendly commit-reveal cryptography.

This resolves the earlier open question of initial token distribution while staying as close as possible to the spirit that tokens should be gained via correct betting.

---

## Concentration, Influence Weighting & Long-Term Dynamics

### Intentional concentration and its risks
Tokens flow primarily to accurate predictors of community norms. This selection pressure is deliberate: people who consistently “read the room” correctly accumulate both immediate rewards (share of opposing stakes) and greater future moderation weight. The same dynamic, however, creates the possibility of whale concentration. A small number of early accurate betters can come to dominate thresholds, making unilateral or near-unilateral action feasible. If those betters later become misaligned with evolving community preferences (“go rogue”), the capital asymmetry slows or blocks correction.

### Quadratic / concave effective weight (decision)
To mitigate whale concentration while preserving skin in the game:

- The **full stake \(S\)** is always at risk and is used for all reward / pot calculations.
- Only the *effective weight* that contributes toward the execution threshold uses a concave function of stake, e.g. \(\sqrt{S}\).

**Reward formula:** The pot is distributed to the winning side’s successful revealers **pro-rata according to the full stake** each of them committed. The concave weight is never used for reward shares.

Consequences:
- Marginal influence on whether an action passes diminishes with additional capital.
- A large holder still loses (or gains) the full amount they staked; they cannot cheaply dominate thresholds without putting real skin at risk.
- Computation remains trivial and mobile-friendly.
- Compatible with the existing secret commit-reveal scheme; the square-root (or equivalent) is applied only at revelation / settlement when computing \(W_{\rm yes}\) and \(W_{\rm no}\).
- Free commitments (set balance = 1) continue to work unchanged and never share the pot.

This softens pure capital dominance without abandoning the core accuracy incentive or reducing the economic cost of being wrong.

### Rogue predictors and community schism
Deep, persistent preference divergence is best treated as a community schism rather than a failure the mechanism must forcibly resolve inside a single room. Clean exit and the creation of a new room (fresh genesis distribution among the departing group) let both resulting communities keep the norms they actually prefer. Because tokens are local to each room, power does not automatically carry over. This exit option is operationally cheap given the open-source, minimal-state design and is consistent with the ephemeral-channel orientation already present in the bootstrap.

### Ephemeral rooms and inflation
Under the working assumption that individual chats / communities are finite-lived — they are born, run for a period, then die, and new ones are started from scratch — continuous inflation or token emission is **not required** for the purpose of diluting whales. The natural end of the room already resets power. New rooms bootstrap independently via the genesis + free-commitment rules.

Mild continuous emission (or other anti-ossification measures) is retained as a **possible future mechanism**, mainly useful if the system is later extended to support longer-lived communities that risk ossifying. It remains an optional refinement rather than part of the core design.

---

*Mechanism design for the Futarchy-based chat moderation system.*

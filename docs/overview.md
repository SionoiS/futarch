# Decentralized Futarchy-Based Chat Moderation — Overview (What & Why)

## Project Overview

A fully decentralized moderation system for chat applications based on **futarchy** principles (Robin Hanson’s “vote on values, bet on beliefs”) using **secret betting**.

Chat participants can place token bets on proposed moderation actions (e.g., “remove this message”, “ban this user”, or other actions). When a proposal accumulates enough stake to cross a threshold, the action is executed and successful betters are rewarded.

The system aims to aggregate distributed information and incentives rather than relying on centralized moderators or simple majority voting.

---

## Incentives

The system is intentionally built so that private incentives push participants toward producing a chat the community itself regards as “good.”

### Primary incentives
- **Desire for a good chat experience**  
  Users want conversations that feel high-quality, useful, and free of the things the community finds disruptive. Because “not good” is defined by the community rather than by fixed absolute rules, the moderation mechanism must continuously discover and enforce evolving community standards.

- **Points as moderation power**  
  Users want to accumulate points (or equivalent reputation / stake weight). Higher points translate directly into greater moderation power — larger effective betting weight, easier ability to create proposals, or stronger influence over thresholds.

- **Accurate prediction of community norms**  
  The main way to earn points is to correctly bet on one’s own ability to forecast what the community will ultimately treat as undesirable. Successful prediction of the eventual consensus outcome rewards the better and increases their future power. Incorrect predictions cost points and reduce influence. This creates a continuous selection pressure for people who are good at reading the room.

### Supporting / secondary incentives
- **Direct economic reward**  
  Winning betters receive a share of the opposing stakes (or pot). This provides an immediate material payoff in addition to the longer-term power gain.

- **Self-correcting influence**  
  Chronic mis-predictors steadily lose points and therefore lose moderation power. Accurate predictors gain it. This reduces the risk that a persistent minority can capture the system.

- **Skin in the game**  
  Every proposal and every bet carries real cost. Frivolous, purely punitive, or low-information moderation actions become expensive, which discourages abuse.

- **Collective preservation of the environment**  
  A healthier, more enjoyable chat retains users and keeps the community valuable. Participants who care about the long-term health of the space have an incentive to bet in ways that protect it.

These incentives are meant to replace both centralized moderator discretion and rigid rule lists with a continuous market in community standards.

---

*Overview of the what and why of the Futarchy-based chat moderation design.  
See `mechanism.md` for the core mechanism and token distribution details.*

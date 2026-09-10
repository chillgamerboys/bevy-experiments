# Gamekit consolidation and balance infrastructure

## Decision

Keep one repository with two product boundaries: `crates/` is the reusable library;
`games/` contains its consumers. Labyrinth is the flagship, Deckbuilder is the
contrasting UI/network/balance adopter, and Carterfight remains a small offline
consumer. Do not create another validation game now. Synthetic test fixtures are
appropriate for contracts that neither real game exercises.

Gamekit is a set of capabilities, not a combat engine. Extract reusable contracts
from Labyrinth, not all of Labyrinth's features. A shared algorithm must not depend
on Labyrinth's entities, abilities, screen organization, rules or content.

The agreed priority is to implement and adopt GameSkills before the broad
documentation and code refactors. The [draft framework sequence](decisions/gameskills-framework.md)
proposes the milestones and keeps the capability work below as scoped follow-on
work. Labyrinth's playable release and Deckbuilder's companion development lead;
the existing-game [Port Vila pilot](decisions/port-vila-adoption.md) is deferred
until after the internal refactor and candidate packaging.

## Implementation sequence and acceptance

### 1. Consumer and distribution boundary — implemented

- `bevy-gamekit` is a thin, feature-gated facade with no default dependencies or
  umbrella plugin. Capability crates remain independently usable.
- All three game applications use the facade; pure game rules stay independent.
- Policy rejects capability dependencies on games or back onto the facade.
- An external consumer builds from Cargo-selected library sources in a temporary
  workspace without games. Separate empty/pure/UI/native feature checks prevent
  workspace feature unification from hiding missing or accidental dependencies.
- CI runs this probe on macOS, Linux and Windows. This is source-consumption
  evidence, not a published-registry install or native-network play test.

### 2. Shared UI and application mechanics — after the skills/docs foundation

Audit existing menu, tooltip, feed, focus and input contracts against both Labyrinth
and Deckbuilder before moving more code. Consolidate only repeated mechanics such
as local menu navigation, bounded overlay lifetime and settings persistence where
their contracts agree. Do not introduce a shared top-level game plugin.

Acceptance: independently test each extracted mechanism with small fixtures; adapt
both games in the same change; retain each game's typed actions, styles, layout,
targeting, ability explanations and log semantics. Run production-plugin tests and
interactive pointer/keyboard/menu/tooltip checks in both games. Carterfight checks
that offline UI use remains lightweight. Passing structural tests is not visual
sign-off.

### 3. Multiplayer orchestration mechanics — separate bounded migrations

Compare host/guest connection state, admission delivery, credential storage,
discovery routing and cleanup across both games. Share lifecycle mechanics behind
explicit schedules and narrow callbacks/data contracts; retain game-owned seat
assignment, readiness, authority, protocol payloads, disclosure and reconnect policy.
Do not create a generic replicated combat model.

First resolve the intermittent Deckbuilder admission-refusal delivery tests seen
in the preceding PR: investigate the close/delivery handshake rather than simply
lengthening sleeps or accepting a missing refusal. Then migrate one contract at a
time with failure-path tests before consolidating callers.

Acceptance: two independent consumer protocols; refusal then clean disconnect;
host/guest leave; fresh-process guest reconnect against a running host; both games'
existing integration checks. Local links and localhost do not replace distinct-host
LAN and tailnet tests. Preserve the six-process Labyrinth regression gate.

### 4. Pure balance harness — independent of UI/network consolidation

Build a Bevy-independent capability for seeded experiments using game-supplied
adapters. Its minimum contract is reset from a scenario and seed, current decision
owner, actor-visible observation, stable legal actions, validated step, and explicit
terminal outcome. Keep complete simulator state available for trusted replay, but
never give policies concealed state or future RNG. Distinguish termination from a
step-budget truncation, invalid policy output and simulator failure.

The runner owns seed suites, budgets, policy invocation, reproducible traces and
paired comparisons. Games own action/state types, transition semantics, scenario
construction, metrics, rewards and cost/level equivalence. Initial policy backends
are scripted, random-legal and simple search/baselines; no Python or RL dependency
is necessary to establish the contract.

Labyrinth supplies its production pure rules as an adapter, not a second combat
implementation. Move enemy decision selection into game-local, deterministic
creature profiles with explicit tie-breaking and seeded randomness when requested.
Profiles can differ in competence, preferences and lookahead; shipping enemies do
not become learned policies. Do not add the future stats/items/progression system
just to populate a harness.

Deckbuilder supplies a small headless adapter over its own rules. Extract its pure
reducer locally if needed; do not expand its content or design a third game to fit
the API. Both adopters are the gate for stabilizing the runner contract.

Acceptance:

- Replaying scenario + seed + commands + versioned rules reproduces the result.
- Scripted and candidate policies consume the same legal-action/observation API;
  concealed inputs cannot affect an observation or leak through diagnostics.
- Paired comparisons use identical seed/scenario suites, report sample counts and
  variation, and distinguish wins, losses, draws and budget truncations.
- A game-defined ability substitution experiment holds level, budget and encounter
  context fixed. Later experiments may allow re-optimized loadouts as a separate
  question. Report damage, survival, action economy, status/position utility and
  outcomes where the game supplies them; do not collapse balance to win rate alone.
- Scenario and policy fingerprints accompany results. Separate tuning seeds from
  held-out evaluation and report weak-opponent exploits, not just aggregate scores.

A learned-policy bridge comes later, only when existing baselines and throughput
justify it. It should wrap the same runner/rules and preserve legal-action masks,
actor observations and terminal/truncation semantics. Learned actors are balance
instruments, not the shipped enemy AI and not proof of fun or perfect balance.

### 5. First private distribution — explicit release task

Distribute the facade and capability packages, plus the canonical skill pack under
one pinned release tag. Exclude `games/`, game assets and game fixtures. No repository
split is required. A source archive and a private registry release have different
requirements; select the channel explicitly before preparing the first release.

Acceptance: library-only builds/tests; reviewed package contents and licensing;
versioned internal dependencies suitable for the chosen channel; documented feature
and platform matrix; clean external install from the actual release artifact; skills
recognize facade consumers as well as direct capability consumers. Do not call the
current source-staging probe a registry release test.

## Extraction guardrails

Reusable candidates: experiment scheduling/reporting, deterministic test mechanics,
connection lifecycle infrastructure, menu/focus/tooltip/feed behavior.

Game-owned: six-rank formations, initiative rolls, status definitions and timing,
ability composition, creature profiles, costs and progression, victory, hidden
information policy, balance scenarios/rewards, scene and HUD organization.

Generic status/effect resolution is not a prerequisite for the harness. Keep it in
Labyrinth until another game demonstrates the same semantics, not merely the same
words. Favor a small independent contract plus two adapters over a universal game
schema. Each migration must leave both games runnable and retain existing tests.

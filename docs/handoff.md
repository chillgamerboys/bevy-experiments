# Historical Labyrinth and GameKit review

This is the preserved PR21 review, not the current agent entry point. Its five
findings were resolved in the [completed follow-up](handoff-followup-plan.md).
Start with the [documentation index](README.md) for current commands, project
direction and refinement work. The original recommendations below remain attached
to their reviewed revision; they do not override later framework decisions.

Reviewed implementation: `8ade27d`, against main `7531607`; September 10, 2026.
Delivery: [PR #21](https://github.com/chillgamerboys/bevy-experiments/pull/21).
This is a targeted review across rules, presentation, session/admission, discovery,
shared UI and distribution boundaries, not an exhaustive audit or a balance sign-off.

## Follow-up status

The five findings below have been implemented in the Podgorica follow-up.
See the [implementation plan and evidence](handoff-followup-plan.md) and
[Labyrinth verification](../games/labyrinth/labyrinth-testing.md). The original
findings and revision-specific evidence are retained below as review history;
they describe `8ade27d`, not the follow-up implementation. No balance-harness or broad
networking consolidation work is included in this correction.

## Original project context

Labyrinth is the flagship. Gamekit is a set of opt-in capabilities, not its engine.
Keep both in this repository, with dependency direction `games/ -> crates/` only.
The current structure does not justify another broad rewrite. Improve the concrete
contracts below, then consolidate one mechanism at a time with a second consumer.
Deckbuilder is sufficient as the contrasting adopter; do not create a third validation
game or expand its content just to manufacture reuse. Carterfight tests offline use.

- [Labyrinth overview and running](../games/labyrinth/README.md)
- [Labyrinth ownership map](../games/labyrinth/labyrinth-architecture.md)
- [Verification commands and evidence](../games/labyrinth/labyrinth-testing.md)
- [Footprints, death saves and corpse policy](decisions/labyrinth-footprints-and-death.md)
- [Consolidation and balance-harness roadmap](gamekit-consolidation.md)
- [Distribution boundary](extraction.md)

Run from the root:

```sh
cargo run -p labyrinth -- --local
# Faster unoptimized development build, if desired (different Cargo artifact):
cargo run -p labyrinth --profile ci -- --local
```

No `--all-features` is needed to play. Multiplayer testers use distinct profiles;
see the game README. The default company is five players filling six spaces.
Changing the wagon to a single-rank class opens a sixth seat. Do not confuse the
six-single-rank test fixtures with the actual default roster.

## What just shipped

- Gatekeeper, Knifehand, Scout, Field Medic, and player-controlled Lantern Wagon.
  The wagon occupies rear ranks 5–6 with one identity, controller, HP pool and turn.
- Enemy ranks: Ash Brute 1, Iron Brute 2, Ossuary Hauler 3–4, Wound Stalker 5,
  Hollow Archer 6. Every original class appears once; repeats remain supported.
  The Hauler can attack over the frontline without stranding the Brutes' abilities.
- Whole-footprint reach/movement, validated life states, provisional hero death saves,
  corpse HP and persistence of eligible status instances. Corpses last three complete
  rounds after their creation round, can be destroyed, retain spaces until removed,
  and do not prevent victory. Dead identities remain for source/event references.
- Rules fingerprint and wire protocol changed. All peers need the same build.

Death saves are deliberately provisional: d20 below 10 fails, three failures kill,
nonzero damage while dying adds a failure, rescue resets failures. Success only holds
on; there is no stabilization/critical-roll/resurrection system. An all-dying party
loses. Permanent death is within the encounter; there is no persistent maze campaign
yet. The wagon's supply/navigation economy, poison, progression and items are not built.

## Code map and boundaries

| Area | Read first | Responsibility |
|---|---|---|
| Pure rules | `games/labyrinth/rules/src/{model,combat,resolve,formation,life,status,preview}.rs` | Validated state, seeded initiative, effects, legality, lifecycle, immediate previews; no Bevy/network |
| Authored content | `games/labyrinth/rules/src/{lib,content,loadout}.rs` | Prototype rosters, stats, rank masks, skill effects/loadouts, compatibility fingerprint |
| Game authority | `games/labyrinth/src/session.rs` | Seats/ownership, ready/start/rematch, live-session replay watermark, interruption policy |
| Network adapter | `games/labyrinth/src/network/{mod,start,admission,discovery,protocol}.rs` | Game-specific composition of transport/security, typed intents and snapshots |
| Viewer UI | `games/labyrinth/src/{presentation,view}.rs`, `src/ui/` | Disclosed facts, forecasts, typed actions, menus/logs, combat layout and explanations |
| Artwork | `games/labyrinth/src/scene/` | Sprite/cutout geometry, hit regions, appearance; never authority |
| Public facade | `crates/bevy_gamekit/` | Feature-gated re-exports; no default umbrella plugin or game dependency |
| Shared networking | `crates/bevy_game_{session,multiplayer,discovery}/` | Pure security, direct transport/persistence, provider-neutral listing/routing respectively |
| Shared UI | `crates/bevy_game_ui/src/{focus,tooltip,menu,feed,metrics,style}.rs` | Interaction, tooltip lifecycle, local navigation, scrolling, semantic metrics/skin |
| Verification | `crates/bevy_game_test/`, `scripts/check_{repo,distribution}.py` | Test mechanics and library-only source-consumer checks, not game assertions |

The balance harness is still a roadmap, not an implemented capability. Start with
a pure seeded runner and game adapters (reset, observation, legal actions, step,
termination versus truncation, versioned traces). Use Labyrinth's real reducer and
a small Deckbuilder adapter. Shipping enemy AI should stay game-local, deterministic
and creature-profile driven; learned actors are for balancing comparable ability
costs/levels, not controlling shipping enemies. Keep status semantics and formations
local until a second game demonstrates the same contract.

## Original findings (resolved by the linked follow-up)

Priorities: P2 = normal correctness/hardening follow-up; P3 = presentation/documentation.
These are intentionally recorded, not silently fixed as part of this handoff.
No merge-blocking regression was identified in the reviewed normal-play paths.

### P2 — password throttle does not enforce five failures per rolling minute

Location: [SessionPasswordVerifier::verify](../crates/bevy_game_session/src/password.rs),
lines 99–122 at the reviewed revision. Existing Gamekit behavior, not introduced by PR21.

After failures at seconds 0, 1, 2, 3 and 4, the cooldown ends at second 34.
An incorrect attempt at second 35 is verified and returns `Rejected`, rather than
`RateLimited`, even though all five failures remain inside the 60-second window.
The pre-hash check consults cooldown/global count, not per-source window count.
An isolated executable using the public API reproduced this result. The current
`fifth_failure_starts_source_cooldown` test actually expects success at second 35.

Next: reconcile the documented five/minute + 30-second cooldown policy, gate before
Argon2 on both conditions, and test 35/60/61-second edges plus other-source/global limits.
Do not mistake this for an authentication bypass: the password/pin are still required.

### P2 — a single peer can consume the global request-drain budget

Location: [host_messages](../games/labyrinth/src/network/admission.rs), lines 225–240;
[drain](../games/labyrinth/src/network/mod.rs), lines 226–227. Existing adapter behavior.

The entire message buffer is drained into a Vec, then only its first 128 requests
are considered. Per-client admission and the 16-request quota are checked afterward.
If peer A sends 128 messages before peer B's valid request in the same tick, B's
request is silently discarded; even A's excess/unauthorized traffic consumed the
global prefix. Repeating the burst can starve another player's turn. Allocation is
also not bounded by the later `take`. This finding is source-traced, not exercised
with a hostile network client during this review.

Next: a bounded, fair per-connection admission queue and explicit overload behavior;
test a noisy peer before a quiet peer, unauthenticated floods, and retry/sequence
semantics. Retain the durable replay watermark. Do not merely raise the constants.

### P3 — destroying a corpse forecasts the living HP maximum

Locations: [ActorSnapshot::health](../games/labyrinth/rules/src/model.rs), lines 392–396;
[ForecastDisplay::build](../games/labyrinth/src/presentation.rs), lines 299–319.
Introduced by the new corpse presentation path.

Reproduced with a valid snapshot: set Ash Brute's living HP to zero and life to
`Corpse { hp: 1, max_hp: 5, created_round: 1 }`; preview Scout's `SnapShot`.
The target transitions from `1/5` to `Removed` with preview health `0/20` because
`health()` falls back to living maximum after removal. ForecastDisplay forwards this
as the maximum; the main target HP explanation uses the wrong denominator. Damage
and formation removal themselves are correct.

Next: explicitly represent corpse-clear forecasts (and preserve the before-pool
maximum when appropriate); add presentation assertions for partial corpse damage,
corpse destruction, ordinary lethal damage and hidden-health disclosure. Do not reuse
living HP as corpse durability.

### P3 — corpse condition help still describes living turn-start timing

Location: [tooltip catalog](../games/labyrinth/src/ui/battle/tooltips.rs), lines 248–265
and 308–330; [Bleed definition](../games/labyrinth/rules/src/status.rs), lines 167–179.
The new corpse path exposes this mismatch in generic condition descriptions.

A bleeding corpse's condition badge links to the generic description "at the start
of your next 3 turns", while the production resolver remaps corpse trigger/duration
clocks to round end (`combat.rs`, around line 666). Corpses never take turns. Instance
help says only "boundaries left", so it does not explain the exception. Source-traced;
the lifecycle behavior itself is tested, but this tooltip wording was not exercised
in the native walk.

Next: disclose effective per-instance timing in the game-owned presentation, explain
the corpse exception, and test living/dying/corpse tooltip content against the same
timing policy used by resolution. Keep content semantics out of Gamekit tooltips.

### P3 — optional skill reference describes superseded tooltip behavior

Location: [Gamekit API reference](../skills/references/gamekit-apis.md), tooltip section.
It still describes delayed previews, gap-tolerant reading and explicit pinning, whereas
the current plugin implements immediate hover, immediate pre-lock dismissal and dwell
locking. Source implementation/tests are authoritative. Refresh canonical prose and
regenerate clients when doing the planned skill rewrite; do not hand-edit generated
client copies. The baseline remains seven skills.

## Evidence and limits

Local macOS checks passed on `8ade27d`: workspace all-feature tests/doctests; 51 rules
tests; 105 Labyrinth application tests; the separately enabled six-process abrupt
guest-kill/restart test; strict Clippy, formatting, dependency policy and repository
link/dependency-boundary checks. Five-App encrypted wagon join/action/fresh-client
reconnect is separate from the explicit six-single-rank capacity/process fixture.

Native rendered review: living formation at 1280×720 and 1920×1080, authored corpse
layout at 1280×720. Native interaction: keyboard ability selection/focus, pointer target
selection and confirmation advancing combat. These do not establish a complete manual
death-save run. Static corpse frames are authored fixtures, not recorded gameplay.
The review build is an ephemeral local app under `target/review`; use Cargo in a fresh
workspace rather than depending on that bundle or `.context`.

CI is defined in `.github/workflows/gamekit.yml`; consult PR21 for the final exact-head
macOS/Linux/Windows and policy results. Previously reported Deckbuilder admission-refusal
flakiness remains an investigation item in the consolidation roadmap; this pass's local
suite passed and did not reproduce it. Do not claim it fixed.

Not revalidated: distinct-machine LAN multicast/Tailscale, blocked-port behavior on
real tester machines, extended native corpse/death-save interaction, cross-platform
visual appearance, or release installation from a private registry. Current library
source-staging checks are not a published-distribution test. CombatDisclosure is only
a presentation seam: Labyrinth currently transmits full cooperative combat state.
Actual hidden enemy information needs recipient-specific snapshots/events/logs first.
No blanket 200% visual-review gate is required; retain existing tests without using
them as a substitute for ordinary-size playability.

## Original follow-up sequence

1. Read the footprint policy and run the game; reproduce the two UI findings.
2. Fix the throttle and request fairness with regression tests before networking
   consolidation. Use Deckbuilder to check common mechanics, not shared game policy.
3. Correct corpse forecasting/timing text, then walk damage, rescue, death, corpse
   damage, decay and large-unit movement interactively.
4. Resume the pure balance-harness slice from the consolidation roadmap. First add
   game-local creature profiles and scripted baselines; RL integration comes later.

No unrelated workspace caches, assets or context were deleted during this handoff.

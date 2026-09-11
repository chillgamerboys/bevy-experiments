# Labyrinth and Gamekit handoff follow-up plan

This is a completed implementation record. Use the [documentation index](README.md)
and [current refinement plan](decisions/gameskills-ci-scope.md) to choose new work;
the original sequence below preserves the rationale and evidence for these fixes.

Status: implemented, September 10, 2026. Source inspection is based
on `e073da9e7b4ec851adf8474f3859507153fee252`, the clean checkout following the
[review handoff](handoff.md). The sections below retain the accepted implementation
sequence and contracts.

## Implementation and evidence

All five corrections are implemented. Password gating checks both rolling counts
before hashing. Labyrinth uses bounded per-connection queues with FIFO/round-robin
dispatch, and both network adopters enable a shared gate before Replicon decoding.
Overflow closes the offending physical connection while retaining reservation and
replay history. Corpse forecasts carry an explicit outcome and preserve the depleted
pool's maximum. A pure effective-clock helper drives status resolution and disclosed
help. The canonical tooltip reference was refreshed and both client layouts were
regenerated under `target/review/skills-followup/` for inspection.

The final workspace suite passed 331 tests/doctests, including 113 Labyrinth
application tests, 52 rules tests and 21 session tests. The separately enabled
six-process abrupt guest-restart test passed. Strict Clippy, formatting, dependency
policy, library-only external consumers, repository checks, and both nine-test
Python tooling suites passed. WebAssembly checks passed for hex, turns, session,
UI and Labyrinth rules. Discovery/multiplayer builds without default features,
the minimal test helper, and standalone UI tests also passed.
The existing Deckbuilder refusal tests passed; no
claim is made that the previously reported intermittent failure was fixed.

Native local play verified damage, Bleed application, enemy corpse creation,
round-end corpse damage, the corrected `3 → 0 / 5` clear forecast and destruction,
formation compaction including the Hauler, a dying Scout's rescue, subsequent death
saves/permanent death, and a whole-footprint wagon/Medic swap. The Scout's corpse
persisted into round 5 after creation in round 3. The Mac locked before its round-6
expiry and window resizing could be checked; those remaining interactive checks
are incomplete. Automated lifecycle tests verify the expiry boundary. Authored
forecast/help frames were reviewed at 1280×720 and 1920×1080 at normal scale.
The compact forecast card uses its existing scroll viewport; the full clear
explanation is visible without scrolling in the 1920×1080 capture.

Cross-machine LAN/Tailscale, remote CI platform results and an additional native
Carterfight startup check are not claimed by this run. The offline consumer's tests
and library-only feature checks passed. No distribution install or skill installation
into another workspace was performed. See the current
[verification notes](../games/labyrinth/labyrinth-testing.md) for remaining limits.

## Scope and working model

Resolve the handoff's two P2 networking findings and three P3 presentation/reference
findings. Work in small, independently reviewable changes, with regression tests
beside each correction. The balance harness, creature profiles, wider networking
consolidation and comprehensive skill rewrite remain in the
[consolidation roadmap](gamekit-consolidation.md).

Keep dependencies flowing from games to capabilities. Labyrinth owns its combat,
seats, commands, interruption policy and disclosed explanations. Shared crates own
password security, transport mechanics and tooltip interaction. Deckbuilder is the
second adopter for any shared transport change; Carterfight verifies offline use.
The default Labyrinth party has five players occupying six spaces. Six-player
network fixtures deliberately use six single-rank heroes.

## Code orientation at the planning baseline

| Path through the application | Ownership and observations |
|---|---|
| [Password verifier](../crates/bevy_game_session/src/password.rs) | `verify` prunes history, checks cooldown/global failures, then hashes. The per-source rolling count is missing from the pre-hash gate. The existing fifth-failure test expects success at second 35. |
| [Network composition](../games/labyrinth/src/network/mod.rs) → [host admission](../games/labyrinth/src/network/admission.rs) → [session authority](../games/labyrinth/src/session.rs) | `network_tick` receives messages, handles admission, applies intents and publishes. `host_messages` collects all requests, considers only the first 128, then checks the 16-request peer quota/admission. `PartyAuthority::apply` owns cached results and an independent sequence watermark, including rejected commands. |
| [Rules preview](../games/labyrinth/rules/src/preview.rs) → [presentation](../games/labyrinth/src/presentation.rs) → [actor UI](../games/labyrinth/src/ui/battle/actors.rs) | Preview uses the real immediate resolver. After corpse removal, `ActorSnapshot::health` falls back to living maximum HP. Presentation already handles newly created corpses, but not the corpse-to-removed transition. The forecast health value also feeds the overlay bar. |
| [Status definitions](../games/labyrinth/rules/src/status.rs) → [boundary resolution](../games/labyrinth/rules/src/combat.rs) → [tooltip catalog](../games/labyrinth/src/ui/battle/tooltips.rs) | Resolution remaps corpse clocks to round end. Actor badges/accessibility already repeat part of that remapping; tooltip instance text and linked definitions do not. |
| [Shared tooltip lifecycle](../crates/bevy_game_ui/src/tooltip.rs) → [optional API reference](../skills/references/gamekit-apis.md) | Immediate preview, immediate dismissal before locking, and one-second default dwell locking are implemented and tested. The reference still describes the previous behavior. Explicit `Pin` requests remain supported. |
| [Deckbuilder networking](../games/deckbuilder_ui/src/network.rs) | Uses the same password verifier and acknowledged admission capability, with its own two-seat policy, attempt binding and private snapshots. Its remote request handler has a different shape and currently reads all requests. |

Also inspected the installed, pinned Aeronet 0.21.0 and Replicon 0.41.1 sources.
The backend drains per-connection transport buffers into Replicon before game
authority runs. Replicon's received-message store exposes observation, but no
public filtering/draining API for an application to retrofit a budget there.
Aeronet supplies a per-transport memory limit, defaulting to 4 MiB; that is distinct
from bounded application decoding, retained requests and per-frame dispatch.

Baseline run during planning:

```sh
cargo test -p bevy_game_session -p labyrinth_rules --profile ci
```

Passed: 19 session tests, 51 rules tests and two doctests. These establish a starting
baseline; the missing regressions have not been added. Application/socket tests and
native interaction were not rerun during this planning pass.

## 1. Enforce both password limits — P2

Change `SessionPasswordVerifier::verify` to gate before Argon2 when any of these
conditions holds: the source has five failures in the rolling window, its cooldown
is active, or the global window contains 30 failures.

Document the existing inclusive cutoff explicitly: attempts aged exactly 60 seconds
remain counted; older attempts expire. For failures at seconds 0–4, attempts at 35
and 60 are rate limited, and an attempt at 61 may be verified. The fifth failed hash
still returns `Rejected` and starts the 30-second cooldown. Rate-limited requests
neither hash nor add failures/extend cooldown. Preserve successful-verification
cleanup of source history and preserve the independent global history.

Extend the verifier's tests to cover correct and incorrect passwords while blocked,
the 35/60/61 edges, a spread-out failure history where cooldown outlasts the source
count limit, another IP continuing to authenticate, global exhaustion across IPs,
expiry recovery and bounded tracker cleanup. Use synthetic `Duration` values.
An internal test with an unusable hash can prove that throttling returns before
hash parsing without adding production instrumentation.

Acceptance: no sixth hash while the source window remains full, no password details
in diagnostics, and existing invitation/reconnect paths remain independent. Record
the precise policy in Rustdoc and [multiplayer operations](multiplayer.md).

## 2. Bound and fairly dispatch inbound requests — P2

This is the largest change. Implement and test the game-owned queue first, then
close the earlier ingress gap before considering the finding resolved.

1. Replace collect-then-take with bounded FIFO queues keyed by the current physical
   connection. Check admitted identity and connection-to-seat binding before a
   request can enter a gameplay queue. Recheck that binding when dispatching.
2. Dispatch queues in round-robin order, preserving FIFO within each connection and
   retaining a cursor between frames. Keep the current ceilings of 16 dispatched
   requests per connection and 128 globally per tick. Start with a pending capacity
   of 128 requests per admitted connection, at most 640 across five guest slots.
   A burst within capacity is retained for subsequent ticks; it cannot displace a
   quiet peer's request. Tests also use smaller budgets to exercise rotation.
3. On queue overflow, stop accepting that connection's commands and disconnect it
   with one bounded overload reason. Do not emit a response for every discarded
   packet. Clear its queued requests on overflow, disconnect, revocation or host
   closure. Keep the reserved identity and durable command watermark for reconnect.
4. Add ingress admission/count/byte limits before the Aeronet backend forwards
   messages to Replicon. Prototype the public scheduling seam after
   `TransportSystems::Poll` and before `ServerTransportSystems::Poll`; do not depend
   on private Replicon buffers. Establish limits for pending sockets and handshake
   traffic separately from admitted gameplay. Retain transport memory limits and
   measure the work already done by that layer separately.
5. If this needs a shared receive-budget capability, make it opt-in in
   `bevy_game_multiplayer` and adopt it in both Labyrinth and Deckbuilder in the same
   change. Keep wire payloads, admission decisions, seats and interruption policy
   in each game. Exercise cross-peer fairness with synthetic connections as well:
   Deckbuilder has only one remote seat.

The ingress prototype must demonstrate a real bound on messages/bytes handed to
Replicon, including unauthenticated bursts, before its API is settled. Select byte
ceilings from the actual supported protocol envelopes and test their boundaries.
A bounded queue after an unbounded decode/collection is insufficient evidence.
Avoid replacing the whole backend or consolidating unrelated lifecycle code.

Sequence contract: only dispatched requests reach `PartyAuthority::apply`; its
existing result cache and watermark continue to decide duplicates and stale input.
Transport-discarded requests have no authoritative result. After reconnect, the
host snapshot supplies the next sequence and current decision; an already applied
request cannot execute twice. Do not invent a transient `RequestResult` rejection
that could later become an accepted result for the same sequence.

Preserve the existing interruption policy: disconnecting an admitted flooder can
pause the encounter while its player is absent. Fair admission/dispatch prevents
silent queue starvation; it does not change that deliberate game policy.

Regression matrix:

- Peer A sends 128 requests before peer B's valid request in one tick; B is served
  in that tick within available dispatch budget. Repeat across ticks and reorder
  connection creation to rule out fixed-prefix priority.
- Unauthenticated traffic and repeated Hello/Persisted/Leave messages cannot consume
  another admitted peer's gameplay quota or prevent bounded handshake progress.
- At capacity, capacity + 1, and reduced global budgets, verify queue bounds,
  bounded overload notification, FIFO order, progress and cleanup.
- Disconnect/reconnect with pending requests, duplicate commands, result-cache
  eviction and rematch preserve identity and the durable watermark.
- Test the deterministic queue/production receive schedule, then send bursts through
  real encrypted Apps to exercise the transport boundary. Keep the existing
  six-process abrupt-restart regression.

## 3. Represent corpse clearing in forecasts — P3

Update `ForecastDisplay` and its UI consumers to represent the life transition
explicitly. Prefer a small presentation-only outcome/pool distinction over asking
widgets to infer it from zero HP or parse summary text.

For `Corpse { hp: 1, max_hp: 5 } → Removed`, show corpse durability `1 → 0 / 5`
and “Corpse cleared; formation closes.” Use the before-state corpse maximum for
that depletion; the removed identity's living maximum is irrelevant. Partial corpse
damage retains the corpse pool. Ordinary lethal damage retains the existing living
HP depletion and separately explains the newly created corpse.

Do not add historical corpse durability to authoritative removed actors just to
render a preview. Keep forecast calculations immediate, non-mutating and subject
to the existing conservative disclosure check. Audit the forecast bar, selected
target summary and accessibility text as consumers of the same projection.

Add presentation/preview assertions for partial corpse damage, corpse destruction,
ordinary enemy death, a dying hero's permanent death, and living damage/rescue.
Check both the target and hidden-health bystanders affected by formation compaction.
Hidden facts must remain unknown in text and bars. Preserve the existing lethal
forecast test that forbids displaying corpse creation as living healing.

## 4. Use effective status timing throughout presentation — P3

Extract the existing clock selection into a pure Labyrinth-rules helper returning
the effective trigger and duration boundary for a definition and bearer life state.
Call it from boundary resolution and the game-owned presentation. For a corpse,
map an existing trigger to round end and the duration boundary to round end; an
absent trigger stays absent. Alive and dying actors retain their authored clocks.
Keep eligibility, persistence, tick order and duration decrement behavior unchanged.

Project this timing into condition help, the existing status badge/accessibility
formatting, and relevant forecast timing text. A bleeding corpse with two remaining
opportunities should describe “2 damage at round end; up to 2 round-end ticks while
the corpse remains.” A dying hero's text should refer to its initiative slot/turn
start without suggesting it can choose an action. Modifier-only conditions should
describe expiry rather than damage ticks.

Update the linked generic Bleed/Condition timing explanations to state the corpse
exception, so drilling into a correct instance card does not reintroduce misleading
turn wording. Keep strings and condition semantics out of Gamekit tooltips.

Tests: use the same effective-clock helper in resolver tests and presentation tests;
cover alive/dying/corpse timing, trigger-free modifiers, retained Bleed identity and
remaining duration, round-end damage before corpse expiry, and disclosure revocation
while help is open. Extend the existing UI test that currently asserts “boundaries
left” and “next 3 turns” to validate the appropriate life-state-specific content.

Compatibility check: status descriptions participate in the content fingerprint.
If the catalog prose changes, update its pinned fingerprint expectation deliberately.
A behavior-preserving helper extraction alone does not require a rules revision;
any actual semantic change does. Change the application schema if wire data changes.

## 5. Refresh the canonical tooltip reference — P3

Update only the relevant section of `skills/references/gamekit-apis.md`: immediate
hover preview, immediate dismissal before lock, continuous hover using `lock_delay`
(one second by default), pointer-transparent preview, locked-card dismissal, explicit
T inspection and nested navigation. Preserve the supported `Pin` API; distinguish
it from the removed visible Pin/footer row and old delayed/gap-tolerant lifecycle.

Use the existing deterministic renderer in `skills/scripts/skills_tool.py` to
regenerate Codex/Claude layouts into a temporary review directory and inspect the
reference in both outputs. Run the skill validator and installer/sync tests. Keep
seven canonical skills; generated client copies are outputs, never prose sources.
Installing updated skills into other workspaces is a separate distribution action.

## Integration, evidence and completion

Implement in the order above: password security, request fairness, corpse forecast,
status timing, reference refresh. Forecast and timing changes can share fixtures,
but should remain independently reviewable. Run focused package/module tests after
each change, then the complete existing verification gates once the set is ready:

```sh
cargo test --workspace --all-features --profile ci
cargo test --workspace --doc --all-features --profile ci
cargo test -p labyrinth --lib network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight --profile ci -- --ignored --exact --nocapture
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo fmt --all -- --check
cargo deny check
python3 scripts/check_repo.py
python3 scripts/check_distribution.py
python3 -m unittest discover -s scripts/tests -v
python3 skills/scripts/validate_skills.py
python3 -m unittest discover -s skills/tests -v
```

Retain the minimal-feature, browser-compatible core/UI and macOS/Linux/Windows
checks in [CI](../.github/workflows/gamekit.yml). Investigate any Deckbuilder
admission-refusal flakiness if reproduced; do not claim this work fixes it without
a demonstrated cause and regression.

Run Labyrinth locally and walk damage, rescue, death, partial corpse damage,
destruction, round-end decay and movement of the wagon/Hauler. Inspect forecasts and
condition links using pointer and keyboard, including after resizing. Review
1280×720 and 1920×1080 at normal scale; preserve existing scale tests without adding
a blanket 200% manual gate. Authored fixtures can make rare states reproducible, but
record fixture review separately from an actual reducer-driven lifecycle walk.

For a shared transport change, also verify Deckbuilder admission/action/reconnect
and Carterfight offline startup. Report local encrypted-App tests separately from
cross-machine LAN/Tailscale tests; the latter remain unverified unless actually run.

Completion means each of the five findings has an implemented correction and
appropriate evidence, the required checks pass, and the handoff/testing documents
record the resulting revision and remaining verification limits. Resume the balance
harness as a separate planned slice after this follow-up is complete.

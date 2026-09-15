# Labyrinth customization: remaining acceptance

Status: active acceptance follow-up; implementation delivered, with broader usability, network and package evaluation still open.
Owners: Labyrinth, Gamekit and GameSkills. Tracking: [HEX-99](https://linear.app/chillgamerboys/issue/HEX-99).

## Delivered scope and current sources

[PR #39](https://github.com/chillgamerboys/bevy-experiments/pull/39) delivered the
content/build/scenario, co-op ownership and editor foundation. PRs #42–45 delivered
subsequent battle/character UI work. These integrations are complete; the old
PR #39 audit/merge checklist is retired. PRs #46 and #48 delivered GameSkills
workflow and usage improvements; #47 changed shared tooltip locking to two seconds.
[PR #49](https://github.com/chillgamerboys/bevy-experiments/pull/49) added comprehensive
backend mechanics coverage and corrected movement across large actors.

Current contracts live in:

- [Labyrinth architecture](../../games/labyrinth/docs/architecture.md),
  [mechanics coverage](../../games/labyrinth/docs/mechanics-coverage.md), and
  [testing](../../games/labyrinth/docs/testing.md).
- [Character editor design constraints](../../games/labyrinth/docs/character-editor-design.md).
- [GameSkills evaluation](../../gameskills/docs/plans/skill-evaluation.md) and
  [measured workflow](../../gameskills/docs/agent-workflow.md).
- [Gamekit architecture](../../gamekit/docs/architecture.md).

The implemented catalog has 34 Skills, three passive Abilities and 11 weapons.
The 209 new deterministic scenarios exercise the authoritative backend; they do
not constitute an RL training adapter, final balance or player usability evidence.
Historical plans and their original acceptance checklists remain available in
[Git at the pre-reconciliation revision](https://github.com/chillgamerboys/bevy-experiments/tree/bf42159a8b2872ca293fafe1a5e5a66eb4f1f666/docs/plans).

## Outstanding acceptance

This table preserves unresolved work from S1–S10; it does not infer current Linear
statuses from code or mark the broader epic Done. Use each new task's actual scope
and receiving policy, rather than reopening every historical check for a docs fix.

| Slice / tracker | Delivered foundation | Remaining evidence or work |
|---|---|---|
| S1 [HEX-100](https://linear.app/chillgamerboys/issue/HEX-100) | Numbered grill, native Codex discovery, scoped rigor, model routing and measured stages | Independent forward behavioral comparisons, Claude/update/recovery/two-pin trials, supported cost conclusions; preserve earlier negative observations |
| S2 [HEX-101](https://linear.app/chillgamerboys/issue/HEX-101) | Validated content, provenance, prerequisites, upgrades and actor-local resources | Inventory/progression remain deferred; passives are now implemented |
| S3 [HEX-102](https://linear.app/chillgamerboys/issue/HEX-102) | Both-team scenarios, reproducible seeds, save/load and external-controller seam | Full Gym bridge, rewards and training remain deferred |
| S4 [HEX-103](https://linear.app/chillgamerboys/issue/HEX-103) | Weapon/learned mechanics, captured-target cleave and forecast/commit tests | Balance remains experimental; backend evidence does not demonstrate human use of every move |
| S5 [HEX-104](https://linear.app/chillgamerboys/issue/HEX-104) | Multi-character ownership, spectators, paused reassignment, stale command rejection and encrypted/process regressions | Distinct-machine LAN/Tailscale discovery, admission, gameplay and reconnect remain separate from localhost evidence |
| S6 [HEX-105](https://linear.app/chillgamerboys/issue/HEX-105) | Spatial preparation, one editor, inspect-before-change, effective build comparison and retained drafts | Complete player journeys and comprehension after prior rejected menu designs; dirty switching, enemy setup, save/load, ready/start and focus during roster updates |
| S7 [HEX-106](https://linear.app/chillgamerboys/issue/HEX-106) | Every granted move, overflow/input, provenance, forecasts and stale selection guards | Native feel and full-list pointer/keyboard inspection, targeting and confirmation; off-turn inspection must not confer authority |
| S8 [HEX-107](https://linear.app/chillgamerboys/issue/HEX-107) | Shared focus/tooltip fixes and affected consumer regressions | Maintain capability findings and actual package-consumer evidence when contracts change; no extraction quota or release claim |
| S9 [HEX-108](https://linear.app/chillgamerboys/issue/HEX-108) | Automated scenario/session/UI checks and historical encrypted six-process reconnect/fight evidence | Candidate-bound human play and available cross-machine routes, distinguished from tests and frames |
| S10 [HEX-109](https://linear.app/chillgamerboys/issue/HEX-109) | Published and integrated implementation PRs | Reconcile broader epic acceptance only after outstanding outcomes are observed; integration alone does not close evaluation |

For S6/S7/S9, exercise edit/load → assign owners → customize → ready/start →
weapon/learned actions → death/rescue → pause/reassign → reconnect → finish/restart.
Include small/full formations, reproducible seeds, multiple owned characters,
spectator disconnect and stale former-owner input. Record actual source, content,
route and observation identities; do not relabel historical evidence as current.
Use the configured normal display for affected journeys; extra platforms/scales
follow explicit release scope. These broader gaps do not silently become blockers
for unrelated feature PRs.

## Integration contracts

- **Content/build:** stable IDs reference versioned game-owned definitions; grants
  retain provenance and upgrades retain their contributions. New content using
  known effects is data-only; new effect semantics are typed/tested Rust. Authoring
  format defaults to TOML. Catalog/resource bounds protect validation/transport
  while permitting every move granted by a valid build, including more than eight.
- **Scenario/replay:** parsed battle input is independent of I/O and Bevy. Same
  canonical content, rules, scenario, seed and ordered actions reproduce behavior.
  Freeze content for the encounter. Save game configuration without credentials
  or machine-specific peer assignments; new lobbies rebind controllers explicitly.
- **Compatibility:** initial co-op requires matching code/rules/catalog identity;
  host sends the authoritative scenario/resolved build view and clients validate it.
  Do not add mod downloading. Change wire/rules versions with action/snapshot/event
  semantics; old clients reject clearly. Review actual payload budgets.
- **Authority:** host owns scenario/enemies/formation/controller assignment; players
  edit owned builds in lobby. Every gameplay command names an actor and current
  decision/assignment context. Stale authority must fail even after assignment
  moves away and back. Preserve sequence replay protection and combat resources.
- **Pause/reconnect:** add host-controlled assignment pause distinct from local
  menus, missing-controller suspension and rules failure. Spectators never gate
  Ready/Start or pause on disconnect. Live/dying assigned heroes preserve the
  controller reconnect requirement until explicit reassignment. Host can spectate
  while retaining session authority. No new host restart or late-join guarantees.
- **UI/controller:** UI and AI consume resolved immutable views and submit the same
  legal action vocabulary. Selecting/inspecting another actor never changes control.
  All active Skills remain accessible in automatic groups/rows; no favorites or
  layout customization. Test policies use the same combat boundary as future RL.
- **Shared packages:** capabilities stay optional and independent of game nouns.
  Assessment begins with implementation and produces findings plus justified fixes,
  not a forced extraction quota. Skill guidance is measured on actual selection,
  execution and endpoint preservation, separately from tests/discovery.

## Deferred work

Inventory storage/acquisition, ammunition/retrieval, offhand/dual wielding,
progression/skill trees, general reactive effects such as retaliation, custom
hotbars, manual enemy-control UI, full Gym/training integration, host migration,
host-process persistence, new mid-combat admission, registry release,
linked-workspace migration and Linear cleanup remain outside the delivered scope.
The implemented passive Abilities are covered in the mechanics matrix; they are
not part of the older blanket passive deferral.

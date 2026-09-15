---
name: plan
description: Start or substantially revise a Bevy game, GameKit or GameSkills task from a goal, existing plan or bug report. Default entry for scoped implementation and delivery; direct debugging, review and other toolbox requests can use their focused skills.
---

# Plan and carry the task

For a settled small edit, use the lean path in
[efficient delivery](../../references/efficient-delivery.md#match-the-workflow-to-the-change):
one delivery record, focused implementation/checks, concise review and publication.
A separate plan document, queue or worker is optional, not a prerequisite.

Read [project context](../../references/project-context.md). Recover the user's
outcome, requested endpoint, accepted choices and current task before planning.
Read the affected owners' current architecture and Decisions sections; inspect
source/callers when a constraint or claimed behavior matters. Use
[creative levels](../../references/creative-levels.md), defaulting to level 2.
Resolve the independent [verification rigor and affected scope](../../references/verification.md#rigor-and-affected-scope)
from the project and actual receiving branch before selecting acceptance checks.

Use `grill` for material unresolved choices; routine implementation choices need
no interview. Record the bounded change, preserved/changed constraints, owners,
steps and relevant verification from the project's development/testing guidance.
Name unfinished requirements. Substantial work may need a committed active plan;
routine notes do not require a plan file, queue or ticket.

When the project opts into model routing or usage tracking, use
[efficient delivery](../../references/efficient-delivery.md). Keep one cohesive
change in one PR unless independent acceptance warrants a split; worker boundaries
do not imply PR boundaries. Prefer prompt integration over a growing dependent stack.

For a new or materially changed player-facing workflow, plan from the player's
goals and frequent tasks before listing controls. Read
[interface design](../../references/interface-design.md) to connect interaction,
information hierarchy, recurring views, state/feedback and authority with a
concrete visual design. Spatial relationships are one consideration, not a default
layout. Accepted mechanics do not establish a screen design. Investigate existing
routes and requested references, then use `grill` for consequential unanswered
choices with alternatives and a recommendation. Make the intended appearance and
complete task flow reviewable before broad production wiring; carry that design
and its acceptance into implementation. A routine fix to a settled view needs
neither a new design exercise nor another approval.

For implementation, read [delivery](../../references/delivery.md) and resume or
start its delivery record. Keep the requested endpoint through interruptions. A
plan-only discussion ends at its plan; focused specialist requests retain their
narrower scope. Optional tracking follows the adopted package and current tools.

Only for an authorized parallel wave, read [work orders](../../references/work-orders.md),
validate a scoped plan with `plan validate --file PLAN.json`, then create one queue
and hand it to `dispatch`. Resolve authorization from the accepted session and
applicable project instructions/configuration, including inherited authorization
after a handoff. Keep task scope separate from execution strategy and authorization;
report unresolved permission as such instead of rewriting scope. Capacity alone is
not permission to launch agents. Solo work stays with the current agent.

Implement, verify through the project's configured checks, reconcile current docs
and proceed through `create-pr`, `audit-pr`, and authorized merge/release work as
the endpoint requires. Invoke skills through the host when available; links alone
are not calls. Preserve concrete missing observations rather than claiming completion
from local commands. Remove a completed plan after its useful conclusions reach
current docs and its whole outcome is settled; do not delete outstanding acceptance.

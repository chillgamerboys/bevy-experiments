---
name: plan
description: Start or substantially revise a Bevy game, GameKit or GameSkills task from a goal, existing plan or bug report. Default entry for scoped implementation and delivery; direct debugging, review and other toolbox requests can use their focused skills.
---

# Plan and carry the task

Read [project context](../../references/project-context.md). Recover the user's
outcome, requested endpoint, accepted choices and current task before planning.
Read the affected owners' current architecture and Decisions sections; inspect
source/callers when a constraint or claimed behavior matters. Use
[creative levels](../../references/creative-levels.md), defaulting to level 2.

Use `grill` for material unresolved choices; routine implementation choices need
no interview. Record the bounded change, preserved/changed constraints, owners,
steps and relevant verification from the project's development/testing guidance.
Name unfinished requirements. Substantial work may need a committed active plan;
routine notes do not require a plan file, queue or ticket.

For implementation, read [delivery](../../references/delivery.md) and resume or
start its solo record. Keep the requested endpoint through interruptions. A
plan-only discussion ends at its plan; focused specialist requests retain their
narrower scope. Optional tracking follows the adopted package and current tools.

Only for an authorized parallel wave, read [work orders](../../references/work-orders.md),
validate a scoped plan with `plan validate --file PLAN.json`, then create one queue
and hand it to `dispatch`. Capacity is not permission to launch agents. Solo work
stays with the current agent.

Implement, verify through the project's configured checks, reconcile current docs
and proceed through `create-pr`, `audit-pr`, and authorized merge/release work as
the endpoint requires. Invoke skills through the host when available; links alone
are not calls. Preserve concrete missing observations rather than claiming completion
from local commands. Remove a completed plan after its useful conclusions reach
current docs and its whole outcome is settled; do not delete outstanding acceptance.

---
name: plan
description: Start or substantially revise a Bevy game, GameKit or GameSkills task from a goal, existing plan or bug report. Default entry for scoped implementation and delivery; direct debugging, review and other toolbox requests can use their focused skills.
---

# Plan and carry the task

Read [project context](../../references/project-context.md). Reuse accepted
investigation, decisions and authorization. Identify the requested artifact and
endpoint before expanding the workflow; a plan-only request ends with a plan.
An implementation/delivery request continues through the applicable authorized
checks and PR workflow after planning. A narrower user request takes precedence
over a project's normal PR endpoint.

Inspect relevant owners and call sites once. Record observed facts, important
source locations, reproduction/design evidence and unresolved material choices.
Apply [creative levels](../../references/creative-levels.md), defaulting to level 2.
Resolve routine engineering choices yourself; distinguish an accepted design from
an experiment. Keep GameKit optional and select craft/package guidance only for
the work at hand.

Define a bounded result, retained/changed contracts, owners, acceptance claims and
the evidence each requires. A solo change needs a compact plan and implementation
by the current agent. Do not create an issue or queue just to satisfy a ritual.

For an authorized parallel wave, read [work orders](../../references/work-orders.md).
Produce self-contained orders and one durable queue; separate start dependencies
from merge dependencies, reserve shared files/resources and include human-owned
work. Validate with `plan validate --file PLAN.json` before `queue create --file
PLAN.json`, using the common runtime prefix. Hand the established wave to
`gameskills:dispatch`; the queue helper does not launch agents.

After implementation, invoke selected checks and documentation work, then
`create-pr`, `audit-pr`, and authorized `merge-pr`/`release` as the endpoint requires.
Load these through the host; links alone do not invoke them. Preserve partial
results and concrete blockers on interruption. Do not call a task complete merely
because its plan, worker launch or implementation step finished.

For material unresolved choices, use `gameskills:grill`; clear tasks skip the
interview. For implementation work, read [delivery state](../../references/delivery.md)
and start or resume the durable solo-task record with the accepted endpoint.
Discover existing records before starting another after an interruption. Use the
configured optional tracking package only when adopted; its absence never excuses
missing publication. Plan-only discussions need no record, queue or ticket.

---
name: grill
description: Resolve material ambiguity in a proposed game, GameKit or GameSkills change before planning. Use when competing goals, ownership, player experience or delivery constraints require user decisions; clear tasks proceed directly to plan.
---

# Resolve the decisions that change the work

Read [project context](../../references/project-context.md). Reuse accepted answers
and investigate repository facts yourself. Frame the outcome, constraints and
largest uncertainty in two or three sentences. Do not require confirmation of a
frame the user already supplied.

Ask only decisions whose answers change scope, architecture, player experience,
risk or delivery. Prefer one to three questions per round, each with a concrete
recommendation and its tradeoff. Resolve upstream choices before dependent ones.
Use the host's available question tool; continue independent read-only research
while waiting. Delegate routine implementation choices to your own judgment.

Track settled, delegated and deferred decisions. State contradictions explicitly
before reopening an accepted answer. Stop when the remaining choices can safely
be made during implementation; do not exhaust a generic questionnaire.

Hand the goal, accepted constraints, decision record, unresolved experiments and
requested endpoint to `gameskills:plan` through the host. Grill itself changes no
code, tickets or external state and is not a mandatory approval or audit gate.

Adapted from jxp's grill at source commit
`b11265ad9681f54a0e365416ce655d279b53f437`, with smaller optional rounds and an
explicit handoff. The wording here is authored for GameSkills.

Read the affected owner docs and Decisions before asking about existing constraints.
Keep new choices in the active plan; update current docs only when the chosen change
is implemented. Resolve contradictory guidance explicitly.

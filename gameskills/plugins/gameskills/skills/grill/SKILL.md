---
name: grill
description: Resolve material ambiguity in a proposed game, GameKit or GameSkills change before planning. Use when competing goals, ownership, player experience or delivery constraints require user decisions; clear tasks proceed directly to plan.
---

# Resolve the decisions that change the work

Read [project context](../../references/project-context.md). Reuse accepted answers
and investigate repository facts yourself. Frame the outcome, constraints and
largest uncertainty in two or three sentences. Do not require confirmation of a
frame the user already supplied.

When a change introduces or reorganizes player-facing views, investigate the
goals and frequent tasks they serve using
[interface design](../../references/interface-design.md). Inspect existing routes
and requested references before interviewing. Challenge consequential assumptions:
does the proposed interaction fit how players choose and compare, what deserves
their attention, and does moving between views lose context or repeat work?
Recover recurring views and authority rather than multiplying editors by entry
point. Agreement on mechanics is not agreement on screen design. Use concrete
visual alternatives and task walkthroughs to expose material gaps in organization,
appearance or behavior; recommend a direction with its cost. Own routine craft
decisions instead of asking the user to design every control or reopening an
accepted answer. Hand off enough design detail to prevent the implementer from
inventing the central experience from a list of fields.

Ask only decisions whose answers change scope, architecture, player experience,
risk or delivery. Prefer one to three questions per round, each with a concrete
recommendation and its tradeoff. Resolve upstream choices before dependent ones.
Give questions visible, stable numbers in both tool prompts and accompanying prose
so the user can answer by number. Continue numbering across rounds; map replies
back to their original decisions and carry settled answers into the plan.
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

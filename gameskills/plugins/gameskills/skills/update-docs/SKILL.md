---
name: update-docs
description: Create or reconcile Bevy/GameKit project explanations, examples, API guidance and migration documentation. Use for documentation work or a changed contract; historical logs and execution queues are not current source documentation.
---

# Reconcile current documentation

Read [project context](../../references/project-context.md). Identify the reader,
affected owners and changed contract. For a refactor, classify each document as
keep, revise, combine, move or retire before editing; extract unfinished work from
mixed historical reports before removing them.

Read actual source, manifests, tests and accepted intent. Current project guides
own local structure, commands, development and troubleshooting. Put consequential
rationale in the relevant page's Decisions section. Public APIs belong in Rustdoc
and examples; reusable craft belongs in packaged references. Resolve contradictions
instead of blindly treating either prose or observed code as the intended contract.

Review affected skill prose and pointers with the docs. Replace duplicate local
facts with targeted navigation that says when to read, what to establish and how
to proceed. Keep skill triggers, necessary judgment and completion criteria.
Installed skills must resolve adopter docs, not this checkout's relative paths.

Update indexes, inbound links, heading anchors and machine readers in the same
change. Distinguish current behavior from proposals and supported operation from
an unverified claim. Run advertised examples when practical and relevant structural
checks through `test`; a prose move needs no automatic native game walk.

Keep substantial active/deferred plans with their owners. At completion, fold useful
conclusions into current guides, preserve outstanding work, remove completed plans
and fix links. Git supplies history; do not create a decision/archive directory or
backup requirement for obsolete narration. If closure needs post-landing observation,
record the small follow-up rather than deleting an unfinished plan.

Carry docs and evidence through the existing task's requested delivery endpoint.
Project doc edits do not authorize publication elsewhere; Bevy upstream retains
its optional human-led contribution rules. Report actual checks and remaining gaps.

---
name: playtest
description: Assess a player journey or compare game-design hypotheses in a Bevy game using observed play and player feedback. Use for clarity, usability, feedback, pacing or feel; pure rules correctness and static UI inspection use test or verify-ui.
---

# Playtest a bounded player experience

Read [project context](../../references/project-context.md) and
[creative levels](../../references/creative-levels.md). Preserve the selected
level; default to level 2. Identify the intended player, journey, success/failure
conditions and the decision this session should inform. Use the actual candidate
build and record platform, controls, route and relevant settings.

Level 1 checks the specified experience. Level 2 finds refinements within it.
Level 3 compares bounded feature alternatives. Level 4 tests scoped concepts
against explicit limits and evaluation criteria. Do not redesign an accepted core
loop while checking a precise interaction.

Walk the complete relevant route, including recovery from mistakes and returning
to a useful state. Observe feedback, discoverability, timing, input and legibility
in context. Use `gameskills-ui:verify-ui` when native focus/rendering evidence is
needed; use domain tests to investigate rules claims separately.

Capture observations and reproduction paths before explaining them. Label agent
interaction, human player feedback and design inference separately. Human enjoyment
requires feedback from actual intended players; clean logs, wins or automated
routes cannot establish it. If no human session is available, record that gap and
continue the observations that can be made.

Prioritize a small set of improvements by player impact and confidence, with
supporting evidence and a way to evaluate them. Implement refinements only within
the active task's authorization; otherwise return concrete options. Preserve
negative results and unresolved hypotheses in the task's acceptance record.

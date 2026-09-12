---
name: author-skill
description: Create or revise a canonical GameSkills skill, precise triggers, focused references and necessary deterministic helpers from observed tasks or failures. Use for skill maintenance; ordinary game documentation and code changes do not automatically require a new skill.
---

# Author from a demonstrated need

Read [project context](../../references/project-context.md). Use the host's available
skill-creator instructions when present. Inspect the actual task/failure, current
callers, candidate source ownership and overlapping skills before adding guidance.
For legacy migration, consult [lesson destinations](../../references/legacy-migration.md).

Choose a discriminating trigger and a bounded responsibility. Put only decision-
changing instructions and essential constraints in `SKILL.md`; move substantial
conditional material into references with clear loading criteria. Prefer updating
the existing owner over creating another planner/test/delivery pipeline. Dispatch
injection is a mode of dispatch, not a second coordination skill.

Preserve the user's scope, existing authorization and client portability. Each
native package's resources must be self-contained. Resolve another installed
package through the host or explicit configuration when a workflow needs it;
never rely on sibling install paths or links escaping to this repository.
Use exact source/Rustdoc references for APIs instead of copied manuals.

Add deterministic scripts only where repeatable behavior warrants them, document
observed interfaces and test them. Metadata should improve discovery and remain
consistent with the body; do not add explicit-only invocation policy unless the
user requests it. Do not edit generated/installed copies as canonical source.

Run available structural validation, inspect reference reachability and test
meaningful behavior through `evaluate-skills` when warranted. Structural success
is not proof of routing, installation or forward performance. Keep useful prior
lessons with a destination or a reasoned retirement, and report validation limits
with the candidate source identity.

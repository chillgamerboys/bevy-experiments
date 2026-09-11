# Portable project context

Read the current repository's instructions, task decisions, manifests/lockfiles
and relevant owner documentation. Preserve existing authorization, local skills
and configuration. GameKit is optional; inspect its exact resolved source and
Rustdoc when present. For Bevy APIs, use the locked version's source/Rustdoc and
official [API documentation](https://docs.rs/bevy/) or
[migration guides](https://bevy.org/learn/migration-guides/), not remembered APIs.

A direct specialist request can run on its own. If an active GameSkills pipeline
needs core configuration, work orders or evidence helpers, resolve `gameskills`
through the host's actual installed skill/plugin location or an explicitly
configured package path. Optional native packages can live in unrelated install
directories: never construct a sibling core path. Read its relevant reference
or invoke its supported skill through the host. Markdown links are not calls.

The resolved core's read-only readiness command is:

```text
<gameskills-executable> --root <repo> status
```

Missing setup or tools remain explicit; do not silently install dependencies or
substitute a source checkout for an installed candidate. Continue useful work
that does not depend on them. The selected creative level is inherited from the
task: 1 implement, 2 refine (default), 3 co-design, 4 bounded exploration. It does
not change engineering rigor, permit new agents or grant publication authority.
Share required checks/evidence with an active core audit instead of rerunning
the same graph for each skill. Results remain tied to actual source and inputs.

# Project context and installed capabilities

Use the current repository, its instructions, the user's accepted decisions and
existing authorization. A resumed task keeps its endpoint and owners. A direct
toolbox request does not require a new plan, an issue, or queue creation.

Resolve the installed `gameskills` package from the host's actual skill/plugin
location, or a project-configured explicit package path. Its directory contains
`skills/` and `references/`. The Rust executable is distributed separately from
the immutable instructions; inspect its `--version` and compatibility requirements. Do not assume a sibling package, a source
checkout or a globally fixed install location is the installed core.

When execution depends on setup, inspect readiness with:

```text
<gameskills-executable> --root <repo> status
```

This is a read-only observation. Missing configuration, packages or prerequisites
must remain visible; it does not install anything. Continue useful independent
investigation, and route deliberate adoption/configuration to `gameskills:setup`.
Use the host's supported invocation/loading mechanism; a Markdown link does not
execute another skill. If an optional skill is unavailable, explain the missing
capability, use available project guidance for the bounded work and preserve any
verification gap. Do not silently install it.

Project-owned `gameskills.toml` selects packages, targets, commands, compatibility,
creative defaults and execution roles. Inspect it along with manifests, lockfiles,
CI and relevant project documentation. Configuration expresses choices; it does
not demonstrate that a command ran or a client/package combination was tested.
Existing `.bevy-gamekit/overlays/` material is local context when migrating an
adopter; preserve its owner and resolve contradictions instead of overwriting it.

Load only relevant craft context:

- [Bevy craft](bevy-craft.md) for ECS, scheduling, state, assets and test altitude.
- [Architecture](architecture.md) for authority or capability-boundary decisions.
- [GameKit source lookup](gamekit.md) when GameKit is present or requested.
- [Creative levels](creative-levels.md) for design latitude and player feedback.
- [Work orders](work-orders.md) for a durable parallel wave, injection or resume.
- [Verification](verification.md) for configured command execution and evidence.
- [Delivery](delivery.md) for PR identity, audit, integration and release claims.

Specialist routing uses logical names resolved by the host, not sibling paths:
UI implementation → `gameskills-ui:build-ui`; UI evidence → `verify-ui` in that
package; discrete game rules → `gameskills-turn-based:model-rules`; networking
boundaries/checks → `gameskills-multiplayer:design-multiplayer` or
`verify-multiplayer`; shared library changes → `gameskills-maintainer:evolve-gamekit`.
Select the contribution package only for a verified engine defect or compelling
engine-level gap worth investigation. Ordinary game/library work has no upstream
stage. Local code fixes do not authorize public communications to other projects.

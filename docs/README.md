# Development documentation

Start with [GameSkills](gameskills.md) for the installed workflow and its current
limits, then [development](development.md) and [testing](testing.md) for commands.
The [framework decision](decisions/gameskills-framework.md) owns project direction;
the [Rust migration plan](decisions/gameskills-rust-cli.md) owns the next tooling
work after the accepted bounded trials. Game-specific behavior belongs with each
game below.

- [GameSkills installation and development workflow](gameskills.md)
- [Implemented CI scope and bounded GameSkills trials](decisions/gameskills-ci-scope.md)
- [Accepted refinement observations and follow-ups](decisions/gameskills-refinement-results.md)
- [Repository-wide Rust tooling migration plan](decisions/gameskills-rust-cli.md)
- [GameSkills and GameKit as a companion to Bevy](decisions/gameskills-framework.md)
- [GameSkills catalog, plan entry and execution pipeline](decisions/gameskills-catalog.md)
- [Deferred draft: Port Vila adoption pilot](decisions/port-vila-adoption.md)
- [Architecture and ownership](architecture.md)
- [Historical PR21 review and resolved findings](handoff.md)
- [Completed handoff corrections and their evidence](handoff-followup-plan.md)
- [Gamekit consolidation and balance infrastructure](gamekit-consolidation.md)
- [Run, build and add a game](development.md)
- [Testing and evidence](testing.md)
- [Multiplayer operations and diagnostics](multiplayer.md)
- [GameKit and GameSkills distribution proposal](extraction.md)
- [Canonical skill pack](../skills/README.md)
- [Focused-workspace decision and recovery baseline](decisions/0001-focused-workspace.md)

Rules and acceptance checks live with [Labyrinth](../games/labyrinth/README.md),
[Carterfight](../games/carterfight/README.md) and [deckbuilder](../games/deckbuilder_ui/README.md).
Public API contracts live in crate Rustdoc and tested examples. This index is not a
second API manual or a collection of old phase plans.

Historical reviews preserve their original revision and evidence limits. Their
old “next” steps are not a current work queue; use the linked completion records
and current refinement plan before treating an old finding as unfinished work.

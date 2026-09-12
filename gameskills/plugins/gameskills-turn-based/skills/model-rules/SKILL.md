---
name: model-rules
description: Model or refine game-owned discrete actions, legality, turn/phase transitions, deterministic replay and test/simulation adapters in Bevy. Use for turn-based rules and sequencing; excludes real-time behavior with no discrete decision owner.
---

# Model deterministic game rules

Read [project context](../../references/project-context.md) and
[rules contracts](../../references/rules-contracts.md). Inspect the authoritative
state and all action producers before changing the contract. Preserve established
rules at level 1; refine clarity/tuning within the accepted experience at level 2.
Design alternatives remain bounded by levels 3/4 and the actual request.

Define stable IDs, typed actions, legality and returned outcomes. Give each mutation
one owner. Separate pure rules from ECS/input/network adapters and project immutable
player views after authoritative mutation. Keep generic roster/cursor sequencing
separate from game-owned effects and victory.

Specify boundary behavior: rejection without mutation, advancement, removal, empty
rosters, phase changes and terminal actions. Control ordering, seeds and time so
replay has an explicit contract. Reuse the same legality/resolution seam for tests,
AI or simulation; do not implement parallel rule engines.

Use source/Rustdoc for the installed GameKit sequencing API if present; do not
require adopting it. For previews or concealed information, apply the relevant
rules-contract guidance on non-mutation and disclosure. Add focused regression
checks at the actual owner and production adapter seam when needed.

Deliver the rules/transition contract or authorized implementation, edge evidence
and unresolved design choices. Passing deterministic rules tests does not establish
presentation clarity, network admission or player enjoyment.

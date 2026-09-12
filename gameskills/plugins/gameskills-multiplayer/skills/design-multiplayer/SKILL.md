---
name: design-multiplayer
description: Define or refine multiplayer authority, visibility, protocol/session boundaries, admission, discovery and reconnect from a Bevy game's requirements. Use for multiplayer design or ownership changes; ordinary local gameplay does not require networking.
---

# Design the required multiplayer modes

Read [project context](../../references/project-context.md) and
[network contracts](../../references/network-contracts.md). Establish supported
player modes, topology, trust/authority, visibility, discovery and restart promises.
Do not make every game adopt multiplayer or add providers outside the request.

Map authoritative owners and dependency direction before choosing protocols.
Distinguish listing, routing, encrypted connectivity, admission, gameplay and
recipient disclosure. Define typed lifecycle/failure states and cancellation/I/O
bounds. Keep provider routes and credentials out of presentation.

Specify action authorization, capacity/seat ownership, protocol compatibility,
stale/replayed-message handling and credential persistence/rotation. Define what
client restart and host restart recover separately. Inspect the exact installed
Bevy/GameKit source for actual composition, scheduling and API seams.

At level 1 follow the settled architecture; level 2 refines its robustness and
clarity within scope. For co-design/exploration compare bounded alternatives and
maintenance tradeoffs against game requirements. An engine-wide proposal is not
a routine outcome of a multiplayer capability.

Use [network evidence](../../references/network-evidence.md) to plan the required
pure, multi-app, socket and actual-machine cases. Return the ownership/protocol
contract, authorized implementation, relevant evidence and unresolved mode or
platform promises. Hand focused verification to `verify-multiplayer` in this
package without starting a second delivery pipeline.

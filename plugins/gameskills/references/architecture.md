# Bevy and capability ownership

Inspect the composition root, plugin graph, resources, states, messages, schedules
and feature graph before moving responsibility. Name one authoritative owner per
mutable domain fact. Other layers read projections or submit typed intent.

Games select plugins, schedules, rules, presentation and adapters. A reusable
capability must not import a consumer game, select its victory conditions or
become its mandatory composition root. Prefer pure data and explicit transitions
when ECS integration is not itself the capability being shared.

Consider extraction when a stable contract is visible from demonstrated needs:
it can be understood independently of a game's nouns, consumers can opt in, tests
do not import a game, and errors/extension seams are explicit. A second consumer
is strong evidence of transfer, not a universal prerequisite for a useful small
package. Keep uncertain abstractions local until the shared behavior is clear.
A convenient tooltip package can remain in GameKit indefinitely; reuse alone is
not a reason to put it in Bevy.

Specify dependency direction and public scheduling seams before a move. Ordering
belongs on real producer/consumer dependencies, usually public system sets, not
plugin insertion order or a global chain. Include lifecycle, cancellation, error
states and test seams. An asset/config failure is not successful default state.

For networking, keep discovery, transport, admission, game authority and disclosure
separate. The optional multiplayer package owns detailed guidance. Game rules,
content, screen-specific views, balance and multi-capability orchestration normally
remain with the game. For a compatibility change, inspect every affected consumer
and feature selection; a default-feature build alone does not establish transfer.

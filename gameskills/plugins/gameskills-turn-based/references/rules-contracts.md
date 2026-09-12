# Game-owned rules and simulation seams

Define stable participant/object IDs, authoritative state, complete typed actions,
legality queries, transitions and terminal outcomes. UI, AI, replay and networking
adapters submit the same game-owned action vocabulary; an adapter must not encode
its own version of the rules or bypass authorization.

A rejected action leaves authoritative state unchanged. Equal initial state,
ordered actions and explicit randomness produce equal transitions. Fix ordering
and seed/time inputs at the domain seam; ECS query or allocation order cannot be
a replay identity. Persist and validate the complete invariant, not just fields
that happen to deserialize.

Keep sequencing separate from effects. Make explicit which event advances a turn
or round, how current/non-current removal behaves, and what empty/terminal states
permit. Rendering, an unrelated removal, previews and failed actions must not
implicitly consume turns or randomness. Test cursor positions, wraparound,
repeated/stale actions, rejection, serialization and terminal behavior where
relevant, including replay after any deduplication cache has been evicted.

A forecast calls the same immediate resolution logic or a verified shared seam
without mutating source state, spending resources or advancing random/turn state.
A preview available outside the current turn grants no commit authority. Project
only disclosed facts: partial concealment may require uncertainty, omitted values
and different projections. Compare views under different hidden states with the
same public facts; inspect derived values, error shape, labels and help as well as
obvious fields.

For GameKit, locate `bevy-gamekit-turns` and `TurnOrder` in the exact resolved crate
source/Rustdoc. Inspect roster/cursor/round, typed advance/removal and serialized
invariants rather than relying on this reference as an API inventory. Pure shared
sequencing should not own cards, mana, movement, combat, AI, timers or victory.
A game may wrap a pure helper in a resource and map its transitions into effects.

Use a pure reducer or narrow game-owned system as the common test/simulation
adapter. Put reusable test mechanics below the game, but retain fixtures and
assertions with the rules they define. A simulation interface is useful when the
task needs it; a balance harness is not a prerequisite for every turn-based fix.

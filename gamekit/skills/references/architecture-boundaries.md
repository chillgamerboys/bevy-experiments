# Architecture Boundaries

## Extraction rubric

Share code only when all of these are true:

- It has a stable capability contract independent of one game's nouns.
- At least two plausible consumers need the same behavior, not merely similar code.
- Consumers can opt in without surrendering their composition root or schedule.
- The shared crate can be tested without importing a consuming game.
- Failure semantics and extension seams are explicit.

Keep code game-local when it encodes genre rules, action legality, balance, victory,
screen-specific presentation models, content catalogs, or orchestration across several
capabilities. Duplicate a small uncertain abstraction until its common contract is
visible.

## Dependency direction

```text
game composition root -> game domain and presentation -> capability crates
capability crate       -X-> consuming game
```

Prefer pure data and returned transitions at the lowest layer. Add Bevy adapters only
where ECS integration is itself the reusable capability.

## Multiplayer and discovery

Keep transport, discovery, admission, and gameplay authority separate. Transport may
own encrypted connectivity, identities, certificate pins, and opaque credentials.
Discovery may publish sanitized availability and resolve provider routes. The game
still owns seats, capacity enforcement, lobby transitions, commands, disclosure, and
rejection policy.

Discovery providers must share one lifecycle rather than leaking mDNS, Tailscale, or
service-specific endpoints into game UI. Treat external routing such as Tailscale as
an adapter dependency, not a shared engine or player identity system. Never place
passwords, bearer invitations, reconnect credentials, or full connection codes in a
discovery record.

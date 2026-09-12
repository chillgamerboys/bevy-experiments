---
name: verify-multiplayer
description: Verify a Bevy multiplayer change across the required processes, transport and network, including admission, disclosure, failures and reconnect. Use for multiplayer acceptance; report actual topology rather than treating mocks or localhost as cross-machine evidence.
---

# Verify each network claim

Read [project context](../../references/project-context.md),
[network evidence](../../references/network-evidence.md) and relevant
[network contracts](../../references/network-contracts.md). Recover promised modes
and the changed boundary; list required scenarios and actual available machines,
providers, binaries, features and topology.

Run deterministic provider/domain cases, production multi-app checks and real
transport scenarios where each is needed. Separately observe listing, routing,
authentication/admission, game authority and per-recipient disclosure. Keep partial
concealment, derived values and serialized payloads in view; a filtered UI is not
evidence of a filtered network snapshot.

For restart recovery, destroy the old client and establish a fresh client loading
stored credentials against a live host. Check stale/replayed attempts and rotation
under the specified protocol. Do not infer host restart support from this case.
Use actual distinct machines for advertised LAN/tailnet paths; if unavailable,
name the gap and continue useful local verification.

Coordinate ports, processes and native resources; bound attempts and collect final
outputs. Share configured command results with the core test/audit graph, retaining
manual topology observations as separate evidence. Changed source, configuration
or environment invalidates affected observations.

Report each scenario's actual setup, pass/failure/incomplete status and artifact,
with uncovered modes/platforms explicit. Simulated services, loopback or successful
transport connection cannot certify a different topology or game admission path.

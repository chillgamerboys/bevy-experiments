# Network scenario evidence

Choose scenarios from promised modes and the changed boundary. Record source,
features/configuration, host/client binary identities, process count, actual
machines/OSes, transport/provider and network topology. Never upgrade a simulated
or localhost result to a cross-machine claim.

| Scenario | Appropriate evidence | What remains separate |
|---|---|---|
| Listing/deduplication/fallback | Deterministic fake provider and typed observations | Real service/multicast behavior |
| Route resolution | Direct and opaque service handoff tests | Reachability, encryption or admission |
| Encrypted/authenticated session | Bounded real transport processes/sockets | Game command authority and recipient filtering |
| Authority/lifecycle | Multi-app action/rejection/disconnect cases | Real network routing if the link is in-memory |
| Recipient disclosure | Inspect per-recipient serialized snapshots under hidden-input variations | UI rendering and other recipients |
| Client restart recovery | Destroy client, start fresh, load stored credentials and rejoin a live host | Host restart recovery |
| LAN/tailnet operation | Distinct-machine discovery, join and gameplay on the named network | Other providers, firewalls and platforms |

Test relevant denial and lifecycle paths: incompatible protocol, locked/full
admission, unauthenticated/replayed/stale commands, disconnect, retry, route loss,
credential rotation and fresh-process reconnect. Keep credential values out of
logs/evidence. Use bounded timeouts, release ports/processes and collect actual
exit/output on failure; a killed test is incomplete.

A same-machine socket test exercises transport code but cannot prove another
machine's firewall, interface routing or multicast. Fake service tests do not
establish Steam integration. Inspect the tailnet adapter's actual supported input
and fixed CLI contract rather than inventing ad hoc routing or shell output.

Coordinate native windows, reserved ports and CPU/memory with active workers. Use
one core command/evidence graph where configured, and attach manual topology walks
as separate observations. Rerun affected checks after source/config/environment
changes. Report missing machines/providers as unavailable evidence while completing
useful deterministic and local transport work.

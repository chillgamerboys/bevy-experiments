# Bevy craft and evidence

Resolve the project's Bevy version, enabled features and toolchain from manifests,
lockfiles and local source. Consult the matching crate Rustdoc and official
[Bevy API documentation](https://docs.rs/bevy/) or
[migration guides](https://bevy.org/learn/migration-guides/) when needed. Select the
actual version; examples from a different release are not a verified API contract.

## Ownership and lifecycle

- ECS entities identify runtime storage; persistent/replay domain identity needs a
  stable game-owned ID. Query or allocation order is not domain order.
- Read the owning plugin and app composition when a helper works in isolation but
  production does not. Register required resources/messages/plugins explicitly.
- Buffered messages, immediate observers and deferred commands have different
  timing. Inspect the installed API and cross the applicable apply/schedule
  boundary before a dependent read. Do not fix timing by adding arbitrary frames.
- Trace state transitions, run conditions and public system sets. Order only real
  data dependencies; plugin registration order is not proof of execution order.
- Distinguish absent, loading, failed and ready. Trace asset roots, launch working
  directory, features and the actual binary before changing content or defaults.
- Keep deterministic seeds, time, ordered inputs and IDs at test seams. Avoid wall
  clock sleeps when a state/frame bound can establish progress.

## Match the test to the claim

| Claim | Useful evidence | Limit |
|---|---|---|
| Rules, invariants, serialization | Pure domain tests | No plugin or renderer wiring |
| Resources, messages, schedule/deferred behavior | Minimal app with relevant production plugins | No rendered appearance or feel |
| UI hierarchy, semantics, focus and layout facts | Structural/headless input checks | No pixel contrast, motion or gameplay truth |
| Static composition, clipping, contrast | Deterministic rendered frames | No timing or interaction |
| Input routes, scrolling, transitions, feel | Native interaction walk/video | No exhaustive domain correctness or human enjoyment |
| Missing diagnostics | Captured logs | Clean logs alone prove no successful experience |

Use the smallest test that reaches the real owner. Convenience plugins and mocks
can conceal missing production wiring; retain an appropriate integration seam.
Assert typed state/transitions before UI text or log strings. Test meaningful
rejection, edge, round-trip and lifecycle cases, not a second copy of the code.
Use configured integration/feature/platform gates when required; an all-feature
workspace suite is not a universal requirement for every adopter or docs change.

For command recording and reuse, read [verification](verification.md). Structural,
rendered and interactive evidence are complementary. A restart claim needs a fresh
process/object and restored state against its real counterpart. Network evidence
must identify whether it used in-memory, localhost or distinct-machine transport;
use the selected multiplayer package for scenario-specific acceptance.

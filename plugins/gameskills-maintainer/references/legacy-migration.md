# Legacy lesson destinations

This ledger records instruction migration from the seven original craft skills and
shared references into the 21-skill candidate. It describes authoring dispositions,
not demonstrated behavioral parity. Resolve named skills/packages through the host;
source-tree paths below are historical provenance, not installed-package links.

| Historical source/topic | Candidate destination or disposition |
|---|---|
| `architect-bevy-game`: single owner, composition root, dependency direction, system-set seams, explicit failures | Core `plan`, `review`, `debug`; core architecture and Bevy craft references |
| Architecture extraction rubric | Core architecture and maintainer capability design; require demonstrated contract, remove universal two-consumer threshold |
| Architecture networking/discovery boundaries | Multiplayer network contracts; core routes only when relevant |
| `build-bevy-ui`: immutable views, typed intent, semantic roles, focus/modal ownership | UI build skill and UI contracts |
| UI fixed 18 logical pixels/44 by 44 minimum | Retired as a universal rule; target criteria come from actual player/platform needs |
| UI untrusted listings, secret buffers, direct-route fallback | UI contracts plus multiplayer authority/disclosure guidance |
| `verify-bevy-ui`: structural/rendered/native split, scope-aware scales, motion | UI verification skill and UI evidence; no automatic maximum-scale gate |
| `model-turn-based-game`: IDs, legality, single mutation, edge cases, sequencing | Turn-based model-rules and rules contracts |
| `debug-bevy-runtime`: exact launch, first wrong fact, asset root, explicit loading errors | Core debug and Bevy craft |
| Debug networking layer/tailnet distinction | Multiplayer contracts/evidence, conditionally routed from core debug |
| `test-bevy-game`: test altitude, minimal production wiring, deterministic bounded progression | Core test and Bevy craft |
| Fixed workspace all-feature integration gate | Moved to project commands/requirements; not universal for all adopters/changes |
| Test multiplayer/fresh restart evidence | Multiplayer verification and network evidence |
| `review-bevy-change`: source identity, owner inspection, relevant lenses, findings | Core review; complete PR acceptance moves to audit-pr |
| `evidence.md`: claim/evidence limits, no log/snapshot overclaim | Core craft/verification plus UI and multiplayer evidence |
| Evidence restart/cache eviction/focus after clipping | Multiplayer lifecycle, turn-based replay, UI contracts/evidence |
| `bevy-0.19.md`: stable IDs, deferred work, ordering, assets | Core craft with locked-version source lookup; UI/network specifics in their packages |
| `gamekit-apis.md`: ownership and integration lessons | Core GameKit source navigator; UI/rules/network/maintainer references |
| Copied GameKit API/default inventory | Replaced by exact resolved source/Rustdoc/examples/contracts; symbols remain search hints |
| Tooltip timing/pinning/disclosure/native hit-testing regressions | UI contracts and evidence; timings come from current source/accepted design |
| Installer/sync pinned source, local overlays, three-way ownership | Core setup and maintainer evaluation; native migration tooling owned by packaging |
| Generated Codex/Claude bodies prove parity | Retired claim: native selection/behavior require real client runs |

Historical project-specific test routes, canvas matrices, tooltip duration and
release commands remain project-owned evidence. Their general failure lessons are
retained above without making those settings requirements for unrelated games.
No old generated installation is automatically removed or rewritten by this
ledger. Native migration and meaningful forward behavior require separate proof.

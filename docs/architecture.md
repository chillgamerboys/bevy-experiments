# Repository architecture

This Cargo workspace develops games while improving Gamekit and GameSkills.

| Owner | Responsibility |
|---|---|
| `games/` | Each game's rules, composition, presentation, assets and acceptance |
| `gamekit/` | Optional library capabilities; see [architecture](../gamekit/docs/architecture.md) |
| `gameskills/` | Portable skills, CLI and workflow guidance; see [architecture](../gameskills/docs/architecture.md) |
| `devtools/` | Internal repository, packaging and CI checks |
| Root configuration and `docs/` | Shared development and coordinated distribution |

## Decisions

Use root-level product directories rather than a toolkit wrapper or a mixed crate
collection. Rust package names use hyphens; Rust imports use underscores. Shared
packages use `bevy-gamekit-*`; game-owned rules stay with their game. The workspace
root is virtual and keeps one dependency lockfile.

Games adopt capabilities independently. Gamekit and GameSkills are companions and
are intended to move together to a future repository; games can move independently.
A split is not necessary to develop or verify release artifacts. See
[distribution](distribution.md) for current contracts and its separate release plan.

Documentation follows the same ownership. Current guides explain supported behavior;
important rationale lives in their Decisions sections. Active plans describe changes.
See [documentation conventions](development.md#documentation-and-skills).

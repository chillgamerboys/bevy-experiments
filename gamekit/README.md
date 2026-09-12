# Gamekit

Optional capabilities for independently composed Bevy games. Games own their
rules, schedules, presentation and assets; Gamekit supplies reusable contracts.
GameSkills is the [development companion](../gameskills/README.md).

| Directory | Capability |
|---|---|
| [facade](facade/README.md) | Feature-gated entry point; no default dependencies or umbrella plugin |
| `hex/` | Pure coordinates and geometry |
| `turns/` | Pure validated roster and sequencing |
| `ui/` | Input, focus, metrics, menus, contextual help and feeds |
| `testing/` | Deterministic Bevy app and UI test helpers |
| `session/` | Identity, credentials and admission security |
| `discovery/` | Public listings and route handoff |
| `multiplayer/` | Secure transport and lifecycle adapters |

Start with the [facade](facade/README.md) for feature selection and Rustdoc for
individual APIs. See [UI integration](docs/ui.md), [multiplayer operations](docs/multiplayer.md),
[workspace development](../docs/development.md) and [distribution](../docs/extraction.md).
The library-only consumer checks exercise Gamekit without the games or GameSkills.

# Bevy Experiments

A workspace for building **Labyrinth** and refining opt-in Gamekit capabilities
through independently composed games. Bevy 0.19; one Cargo workspace.

| Game | Purpose | Run from the repository root |
|---|---|---|
| [Labyrinth](games/labyrinth/README.md) | Primary game: six-player cooperative positional combat | `cargo run` |
| [Carterfight](games/carterfight/README.md) | Pixel-art dialogue battle; contrasting UI adopter | `cargo run -p carterfight` |
| [Deckbuilder](games/deckbuilder_ui/README.md) | Runnable UI and multiplayer regression example | `cargo run -p deckbuilder_ui` |

For a local Labyrinth battle controlling all six heroes: `cargo run -- --local`.
Ordinary play does not require `--all-features`. The multiplayer menu opens no host
until requested. See each game's README for controls and supported behavior.

## Development

```sh
cargo test --workspace --all-features --profile ci
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
python3 scripts/check_repo.py
```

The CI profile optimizes compile time; normal `cargo run` uses the dev profile.
Switching profiles/features can compile additional artifacts. Generated builds and
review captures live under `target/`; do not commit them or admission credentials.

- `crates/`: independent geometry, turns, UI, testing, session, discovery and transport.
- `games/`: each game owns its composition root, rules, presentation and assets.
- `skills/`: the canonical seven-skill Bevy pack and deterministic maintainer tools.
- [Documentation](docs/README.md): architecture, onboarding, testing and diagnostics.

Gamekit is not a shared engine. Reuse stable algorithms and infrastructure without
moving genre rules or orchestration into the library. New games go in `games/`
following the [adopter checklist](docs/development.md), not into a shared game plugin.

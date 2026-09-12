# Bevy Experiments

A workspace for building **Labyrinth** and refining opt-in Gamekit capabilities
through independently composed games. Bevy 0.19; one Cargo workspace.

| Game | Purpose | Run from the repository root |
|---|---|---|
| [Labyrinth](games/labyrinth/README.md) | Primary game: six-player cooperative positional combat | `cargo run` |
| [Carterfight](games/carterfight/README.md) | Pixel-art dialogue battle; contrasting UI adopter | `cargo run -p carterfight` |
| [Deckbuilder](games/deckbuilder/README.md) | Runnable UI and multiplayer regression example | `cargo run -p deckbuilder` |

For a local Labyrinth battle controlling the whole company: `cargo run -- --local`.
Ordinary play does not require `--all-features`. The multiplayer menu opens no host
until requested. See each game's README for controls and supported behavior.

## Development

```sh
cargo test --workspace --all-features --profile ci
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo run --locked -p repo-devtools --profile ci -- check
```

The CI profile optimizes compile time; normal `cargo run` uses the dev profile.
Switching profiles/features can compile additional artifacts. Generated builds and
review captures live under `target/`; do not commit them or admission credentials.

- [Gamekit](gamekit/README.md): opt-in Rust capabilities and their documentation.
- [GameSkills](gameskills/README.md): CLI, canonical plugins and legacy compatibility.
- `games/`: independent adopters, each owning its rules, presentation and assets.
- [Developer tools](devtools/README.md): internal checks, packaging and CI selection.
- [Documentation](docs/README.md): workspace development and cross-product decisions.

Gamekit is not a shared engine. Reuse stable algorithms and infrastructure without
moving genre rules or orchestration into the library. New games go in `games/`
following the [adopter checklist](docs/development.md), not into a shared game plugin.
The [consolidation plan](docs/gamekit-consolidation.md) tracks the library boundary,
adopter validation and upcoming pure balance harness. Games stay in this repository
but are excluded from library distribution.

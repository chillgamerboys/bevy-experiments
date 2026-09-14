# Bevy Experiments

A workspace for building **Labyrinth** and refining opt-in Gamekit capabilities
through independently composed games. Bevy 0.19; one Cargo workspace.

| Game | Purpose | Run from the repository root |
|---|---|---|
| [Labyrinth](games/labyrinth/README.md) | Primary game: six-player cooperative positional combat | `cargo run --locked -p labyrinth --profile ci` |
| [Carterfight](games/carterfight/README.md) | Pixel-art dialogue battle; contrasting UI adopter | `cargo run --locked -p carterfight --profile ci` |
| [Deckbuilder](games/deckbuilder/README.md) | Runnable UI and multiplayer regression example | `cargo run --locked -p deckbuilder --profile ci` |

For the offline Labyrinth setup lobby, add `-- --local`. The main menu opens
no host until requested. Ordinary play does not require `--all-features`.

Start with [setup and launch](docs/setup-and-launch.md): prerequisites, exact game
commands, window sizing, build profiles and optional GameSkills setup. Labyrinth
requests a 1920×1080 logical window with Auto UI scale; desktop limits may apply.

## Development

Select checks for the changed behavior using [testing scope](docs/testing.md).
Logic fixes need relevant owner tests; UI checks and end-to-end journeys follow the
affected surface. The broad workspace suite is not the development default.

The quickstart uses Cargo's `ci` profile for faster local builds. For smoother play
when performance matters, use `--profile dev`; changing profiles/features builds
separate artifacts. Generated builds and captures live under `target/`.

- [Gamekit](gamekit/README.md): opt-in Rust capabilities and their documentation.
- [GameSkills](gameskills/README.md): CLI, canonical plugins and legacy compatibility.
- `games/`: independent adopters, each owning its rules, presentation and assets.
- [Developer tools](devtools/README.md): internal checks, packaging and CI selection.
- [Documentation](docs/README.md): workspace development and cross-product decisions.

Gamekit is not a shared engine. Reuse stable algorithms and infrastructure without
moving genre rules or orchestration into the library. New games go in `games/`
following the [adopter checklist](docs/development.md), not into a shared game plugin.
The [consolidation plan](gamekit/docs/plans/capability-development.md) tracks the library boundary,
adopter validation and upcoming pure balance harness. Games stay in this repository
but are excluded from library distribution.

# bevy-experiments

A sandbox for iterating on small Bevy game ideas. The original Bevy 0.18 experiments
remain independent under `src/games/`. The isolated [Gamekit workspace](gamekit/README.md)
contains opt-in Bevy 0.19 capabilities and the deckbuilder multiplayer adopter.
Games own their composition roots and rules; Gamekit is shared code, not a shared engine.

## Games

| Name      | Run                          | Docs                                                  |
|-----------|------------------------------|-------------------------------------------------------|
| tactics   | `cargo run --bin tactics`    | [docs/tactics](docs/tactics/)                         |

`cargo run` with no args also launches `tactics` (set via `default-run` in `Cargo.toml`).

## Layout

```
src/
├── lib.rs               # exposes `games`
├── bin/
│   └── tactics.rs       # thin wrapper → games::tactics::run()
└── games/
    ├── mod.rs
    └── tactics/
        ├── mod.rs       # pub fn run(); AppState, TurnState
        ├── components.rs
        ├── resources.rs
        ├── systems.rs
        └── constants.rs

tests/
└── tactics/
    ├── main.rs          # mod movement; mod integration;
    ├── movement.rs
    └── integration.rs

docs/
└── tactics/             # phase-by-phase learning notes + testing docs
```

## Adding a new experiment

To add a game called `foo`:

1. Create `src/games/foo/` with its own `mod.rs` exposing `pub fn run()` and whatever submodules it needs (`components.rs`, `systems.rs`, etc.).
2. Add `pub mod foo;` to `src/games/mod.rs`.
3. Create `src/bin/foo.rs` with one line: `fn main() { bevy_experiments::games::foo::run(); }`.
4. Create `tests/foo/main.rs` declaring whichever test modules you want (e.g. `mod movement;`), plus the matching `tests/foo/<name>.rs` files.
5. Add a `[[test]]` entry to `Cargo.toml`:
   ```toml
   [[test]]
   name = "foo"
   path = "tests/foo/main.rs"
   ```
6. Add a `docs/foo/` directory and a row to the table above.

Copy whatever you need from other games rather than refactoring them — the point of keeping experiments separate is to let each iterate independently. Shared abstractions wait for a later cleanup pass.

## Running

```bash
cargo run --bin tactics    # launch the tactics game
cargo test                 # run all per-game test bundles
cargo test --test tactics  # just the tactics tests
```

## WSL2 setup notes

For graphics to work under WSL2 you need Mesa Vulkan drivers (Bevy uses wgpu, which needs Vulkan):

```bash
sudo apt-get install -y mesa-vulkan-drivers vulkan-tools
```

WSLg handles the windowing — `DISPLAY` and `WAYLAND_DISPLAY` are set automatically.

## Tech

- **Bevy 0.18** — ECS game engine
- **Rust 2021**

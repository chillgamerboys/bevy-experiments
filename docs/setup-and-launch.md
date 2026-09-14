# Setup and launch

Run commands from this checkout's repository root. Building a game from source
needs Rust and the platform's native build tools. Playing it does not require
installing GameSkills or launching an agent. On macOS, install the Xcode command-line tools if the linker is
missing. Rustup reads `rust-toolchain.toml` (currently Rust 1.97.1); `--locked` uses
the committed Cargo dependency versions.

## Launch a game

```sh
# Labyrinth main menu; no multiplayer host starts automatically.
cargo run --locked -p labyrinth --profile ci

# Offline battle setup lobby; control the whole company locally.
cargo run --locked -p labyrinth --profile ci -- --local

# Other independently runnable adopters.
cargo run --locked -p deckbuilder --profile ci
cargo run --locked -p carterfight --profile ci
```

The explicit package works from the root or that game's directory. Plain
`cargo run` at the root selects Labyrinth, but uses Cargo's `dev` profile. These
quickstart commands consistently use `ci` for faster iteration. `--profile dev`
uses the more optimized development build when runtime performance matters;
switching profiles or features can build another set of artifacts. Neither Cargo
profile selects GameSkills Development/Testing/Release rigor. Ordinary game launch
needs no `--all-features`, test suite or screenshot review.

Labyrinth requests a **1920×1080 logical window**, with Auto UI scale. Use an
explicit smaller size on a constrained desktop:

```sh
cargo run --locked -p labyrinth --profile ci -- --local --window-size 1440x900
cargo run --locked -p labyrinth --profile ci -- --help
```

The desktop/window manager may constrain the actual size. OS display scaling
determines physical framebuffer pixels; `1920x1080` is not a claim about a Retina
screenshot's pixel dimensions. GameSkills' verification display is a test target,
not a runtime configuration file consumed by the games. Exact offscreen review
dimensions use the [existing capture command](../games/labyrinth/docs/testing.md#static-frame-review).

Deckbuilder still requests 1440×900; Carterfight requests 1280×720 for its authored
pixel-art game. Their normal verification cases are independently selected at
1080p. Their [game guides](README.md) describe controls and scope.

## Build once, run the matching executable

```sh
cargo build --locked -p labyrinth --bin labyrinth --profile ci
./target/ci/labyrinth --local
```

The corresponding `dev` binary is `target/debug/labyrinth`; `release` uses
`target/release/labyrinth`. Do not assume an existing executable contains the latest
source. Rebuild that package/profile before launching it after source changes.
Carterfight resolves assets to its source package's `assets/` directory at build
time: launching from another directory works while that source remains available;
copying only the executable is not a packaged game distribution.

For concurrent Labyrinth instances, assign distinct profiles:

```sh
./target/ci/labyrinth --profile host
./target/ci/labyrinth --profile guest-a
```

Labyrinth's **application** `--profile` goes after Cargo's `--`; it is separate
from Cargo's **build** `--profile ci`. The application profile isolates reconnect
storage and holds an exclusive lock. If the default profile is already running,
use its existing window or choose another profile. Do not delete its profile data
or reconnect credentials to make another launch succeed. `--data-dir PATH` selects
an explicit application-data root; keep the same root/profile for reconnection.
Close the window or use Ctrl-C in its launch terminal to stop that instance.

## Set up GameSkills for agent work

Build a workspace-local CLI; a global installation is optional:

```sh
cargo build --locked -p gameskills-cli --profile ci
./target/ci/gameskills --version
./target/ci/gameskills status
./target/ci/gameskills verification resolve --base dev
```

The executable is `target/ci/gameskills`. Commands written as `gameskills` in the
workflow guides assume that executable is installed on PATH or that this prefix
is substituted. Do not rely on a previous agent's `.context/` executable.

`status` requires a hydrated installation. If its bundle is absent, follow
[installation and existing-pin hydration](../gameskills/docs/installation.md#this-checkout-and-existing-pins).
Inspect `setup` before applying it: bare `setup --apply` selects the CLI's embedded
candidate and can update an older pin. Registration, bundle validity and discovery
in an actual agent session are separate observations. `native codex --launch` and
`native claude --launch` launch agents, not games.

In Conductor, use the workspace terminal for these commands. No shared repository
setup/run script is currently checked in. Each worktree needs its own hydrated
GameSkills bundle and machine-local registration; copying another worktree's
absolute `.codex/config.toml` paths is insufficient. New plugin instructions require
a fresh host/session after verified registration, not merely a rebuilt game.

## Hydrate this repository's retained pin

The committed lock matches the checked-in immutable dev.5 archive. After building
the CLI, inspect and apply the matching installation:

```sh
./target/ci/gameskills setup --bundle gameskills/cli/bundle/instructions.tar.gz
./target/ci/gameskills setup --bundle gameskills/cli/bundle/instructions.tar.gz --apply
./target/ci/gameskills status
./target/ci/gameskills native codex --verify-project
```

The expected content SHA-256 is
`6ab13e01c1e39f189f74fce2041684fe502336cf973434cd37d13099291e68c3`.
`lock_change` should be false for this checkout's unchanged lock. This restores the
pin and local registration. Native discovery is checked separately; open a fresh
host session to use newly registered instructions. Preserve previous bundles and
local overlays. Update this recipe whenever the committed pin changes.

## Current rollout state

PR #46 completed the framework rollout into `dev` at
`d24edcc154a403272fa62114aee6f28bf025ea76`. `dev` is now the GitHub default and
feature-delivery branch, using Development rigor. `main` remains the explicitly
approved milestone branch at Testing rigor. Required PR CI passed on macOS; the
synced merged checkout also passed focused workflow verification and delivery
observation. The [agent workflow guide](../gameskills/docs/agent-workflow.md) owns
model routing, usage accounting and prompt feature integration.

The source CLI and committed instruction pin are `0.1.0-dev.5`. Supported setup
preserved adopter configuration and local overlays, and an ordinary Codex process
discovered all selected packages. This does not reload an existing Conductor
conversation: start a fresh host session to load the revised instructions. Claude
behavior and broader comparative cost trials remain separate evaluation work.

The [evaluation plan](../gameskills/docs/plans/skill-evaluation.md) retains those
trials and the measured tooltip-delay pilot. Release publication, additional
platform/display commitments and linked-adopter migration remain separately scoped.

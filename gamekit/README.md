# Bevy Gamekit Incubator

This nested workspace incubates opt-in Bevy 0.19 capabilities without changing
the legacy Bevy 0.18 experiments in the repository root. Each game remains its
own composition root and owns its rules, domain state, schedules, presentation
models, and typed intents.

Run Gamekit commands from this directory:

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p deckbuilder_ui
```

The deckbuilder now demonstrates a listen host, a two-seat ready lobby, secure
`BGN1` direct codes, rotating reconnect credentials, automatic LAN discovery,
and opt-in development tailnet discovery. See [Multiplayer development and
testing](docs/multiplayer.md) before testing across machines.

Capture a deterministic rendered review frame at an exact logical size:

```sh
cargo run -p deckbuilder_ui --example review_capture -- \
  target/deckbuilder-review.png 1920 1080
```

Pass `200` as the scale argument to review 200% semantic scaling. The capture
uses an offscreen target, so its dimensions are not constrained by the host
desktop.

Append `match`, `multiplayer`, `host`, or `browser` after the scale argument to
capture that route; `match` is the default.

## Skill pack

The seven canonical Bevy 0.19 craft skills live in `skills/source`. The
maintainer workflows in `skills/maintainer` audit and render that source into
both Codex and Claude layouts. Installers must use an explicit release tag or
commit and preserve game-specific conventions in
`.bevy-gamekit/overlays/<skill-name>.md`.

## Future extraction

When the APIs have been proven by multiple games, preserve the incubator's
history in a standalone private repository:

```sh
git filter-repo --path gamekit --path-rename gamekit/:
git remote add origin <private-bevy-game-library-url>
git push -u origin main
git tag v0.1.0
git push origin v0.1.0
```

Consumers should pin a release tag. Do not depend on a moving branch.

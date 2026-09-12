# bevy-gamekit

A thin entry point for independent capabilities. No default features and no
umbrella plugin: games own composition, rules, assets, schedules and presentation.
The capability crates can still be consumed directly, including by pure Rust rules.
This package is incubating privately; it is not yet a registry release.

Inside this repository:

```toml
[dependencies]
bevy-gamekit = { workspace = true, features = ["ui"] }

[dev-dependencies]
bevy-gamekit = { workspace = true, features = ["testing-ui"] }
```

Use `bevy_gamekit::ui::GameUiPlugin` in the game's own Bevy `App`. Enabling a
feature makes APIs available; it does not install plugins or start services.

| Feature | Module / additional behavior |
|---|---|
| `hex` | `hex`: pure coordinates and layout |
| `turns` | `turns`: pure participant sequencing |
| `turns-serde` | `turns` plus validated persistence |
| `ui` | `ui`: semantic controls, focus, menus, tooltips and feeds |
| `session` | `session`: identities and admission security |
| `multiplayer` | `multiplayer` and `session`; no direct transport enabled |
| `direct` | `multiplayer` plus certificate-pinned native WebTransport |
| `discovery` | `discovery` and `session`; no native provider enabled |
| `mdns` | `discovery` plus native LAN provider |
| `tailscale-cli` | `discovery` plus development-only tailnet provider |
| `testing` | `testing`: deterministic minimal Bevy app helpers |
| `testing-ui` | `testing` plus headless UI helpers and `ui` |

With no features, this crate has no dependencies. `hex` and `turns` do not pull in
Bevy. `ui` does not enable transport or discovery. Native discovery still requires
explicit runtime opt-in. Keep testing features in dev-dependencies.

Labyrinth and Deckbuilder exercise different game policies; neither ships inside
this package. Re-exporting a capability is not a claim that all its integration
contracts are mature or that multiplayer has been tested on a real LAN.

# Find the actual GameKit contract

GameSkills works in a Bevy project with no GameKit dependency. Do not add GameKit
unless the task calls for it. When it is used, inspect Cargo manifests and locked
source identity; names and remembered APIs do not establish the installed version.
Resolve path, registry or Git dependencies with Cargo metadata. Read crate/module
Rustdoc, public types, examples and relevant contract tests in that exact source.
Generated local Rustdoc is useful when navigating re-exports or feature gates.

Useful source search terms (these are navigation hints, not an API specification):

| Capability | Search in the resolved source | Contract to establish |
|---|---|---|
| Facade/features | `bevy-gamekit`, `bevy_gamekit`, Cargo features | Which opt-ins activate capabilities and services |
| Native UI | `bevy_game_ui`, `GameUiPlugin`, `GameUiSystems` | View/intent ownership, activation order, focus and semantic metrics |
| Context/tooltip | `UiContextHelp`, `UiTooltip`, `captures_keyboard` | Source eligibility, disclosure, timing, focus, dismissal and input containment |
| Test mechanics | `bevy_game_test`, `TestAppBuilder`, `HeadlessUiPlugin` | Exact plugins/features, frame progression and bounds |
| Discrete sequencing | `bevy_game_turns`, `TurnOrder` | Roster/cursor/round transitions, removal and serialization |
| Session/networking | `bevy_game_session`, `bevy_game_multiplayer` | Identity, credential persistence, authorization and reconnect |
| Discovery | `bevy_game_discovery`, `DiscoveryJoinRoute` | Sanitized observations, provider lifecycle and transport handoff |
| Hex algorithms | `bevy_game_hex` | Coordinate/layout math without board rules |

If source and prose disagree, verify behavior in the owner and its tests, then
repair the authoritative documentation as part of the authorized change. Do not
copy whole API inventories into skills. Record the exact dependency revision and
symbols relevant to the task in its plan/evidence so later work can recheck them.

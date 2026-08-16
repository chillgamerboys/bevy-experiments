---
name: build-bevy-ui
description: Build or refactor native Bevy 0.19 UI with semantic themes, responsive layouts, keyboard focus, accessibility, cards, HUDs, action rails, scroll areas, and blocking modals. Use when implementing game menus or runtime interfaces with bevy_ui. Do not use for egui tooling, non-Bevy frontends, or purely visual asset generation.
---

# Build Bevy UI

Build presentation from immutable views and return typed intent to the game.

Read `.bevy-gamekit/overlays/build-bevy-ui.md` first when it exists; treat it as adopter context without weakening accessibility or evidence requirements.

## Workflow

1. Define a game-local presentation model and typed action/intent enum before spawning entities.
2. Render named semantic regions and controls from that model. Do not query combat, world, inventory, or networking authority inside UI rendering.
3. Use semantic color, typography, spacing, and control roles. Games own branding; reusable UI owns mechanics.
4. Derive responsive layout from logical canvas and semantic density, not physical pixels. Collapse secondary regions before primary actions.
5. Keep essential text at least 18 logical pixels and interactive targets at least 44 by 44.
6. Give every control a stable `Name`, accessible label, logical tab position, pointer activation, and Enter/Space activation.
7. Remove hidden or disabled controls from focus. Trap focus in the highest blocking modal, restore prior focus on close, and keep focused scroll content visible.
8. Validate structure, rendered frames, and interaction separately using `../../references/evidence.md`.

## Gamekit Awareness

When `bevy_game_ui` is present, read `../../references/gamekit-apis.md` and map `UiActivated.entity` through a game-local typed component. Never put consumer action enums or presentation models in the shared crate.

Read `../../references/bevy-0.19.md` before using focus, messages, hierarchy, or UI scheduling APIs.

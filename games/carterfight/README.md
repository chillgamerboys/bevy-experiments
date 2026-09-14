# Carterfight

A small, single-player battle and a contrasting Gamekit UI adopter: authored
pixel art, a pixel font, transparent scene overlays and a white narration panel.
It intentionally has no multiplayer dependency.

## Run

From the repository root: `cargo run -p carterfight`. From this directory:
`cargo run`. Assets resolve to this package's `assets/` directory in both cases.
The initial cold build includes 2D rendering and WAV audio; an all-feature build
is not required. On Linux, development packages for ALSA, udev, Wayland and
xkbcommon are needed (the workspace CI installs them).

## Controls and rules

- Click a move or press 1–3 to select it; selection does not spend a turn.
- Click Confirm or press Space/Enter after selection to commit. Selecting a move
  moves keyboard focus to Confirm. Escape or Cancel clears the selection.
- Tab/Shift-Tab navigates enabled controls; Enter/Space activates the focused
  control. With no focused control, Space/Enter performs the current main action.
- While narration is typing, Show line/Space/Z reveals the line. A separate
  Continue/Space/Z acknowledges it. Prompts advance automatically after typing.
- UI toggles Auto/200% semantic scaling. Motion off reveals text immediately,
  but still requires acknowledgement of narration. Sound toggles the chime
  separately. Scroll or focus controls to reveal overflow at larger text sizes.
- Moves brings the choices into view without changing the selection. The
  selected move's explanation stays in the dialogue panel beside the fixed
  confirmation controls, even when the battlefield content requires scrolling.
- The final Close game button or Space after the outro exits.

Both characters begin with 60 HP. Jab deals 8, Haymaker 20, Headbutt 14. The player
acts first; Carter chooses Jab. Defeated characters cannot act. No rules were
changed as part of adopting shared UI.

## Ownership

`src/backend/` is pure Rust and owns all combat state, moves, damage and ordered
events. `frontend/sequencer.rs` owns game flow, the narration queue and displayed
HP: authoritative resolution happens immediately, but HP presentation advances
only when its corresponding damage event is narrated. The immutable
`CarterfightView` feeds both UI and the world-space sprite health display.

`frontend/systems.rs` maps shared activations and shortcuts to one game-local
intent per frame. `frontend/dialogue.rs` composes this game's scene and skin.
Gamekit supplies focus, activation, accessible metrics and optional control
painting; it does not own the dialogue, rules, camera or game flow.

## Verification

```sh
cargo test -p carterfight --all-targets --profile ci
cargo clippy -p carterfight --all-targets --profile ci -- -D warnings
cargo run -p carterfight --example carterfight_review --profile ci -- target/review/carterfight-1920-auto.png 1920 1080 auto battle
```

Capture arguments are output, width, height, `auto`/`200`, and
`intro`/`battle`/`damage`/`outro`. Changed UI flows use 1920×1080 Auto for
Development/Testing. Logic-only fixes need focused rules checks, not an agent UI
walk. The `carterfight-ui-normal` suite selects normal-display behavior; retained
compatibility cases cover other sizes/scales when the defect or Release support
requires them. Captures drive local typed intents and are static evidence only.
Follow the shared [rigor and milestone policy](../../docs/testing.md).

The original seven backend tests remain; determinism now compares exact ordered
event contents. Additional tests cover narration timing, separate reveal/advance,
selection/confirmation, full intro→battle→outro, duplicate input, native control
activation and six viewport/scale layouts. These do not prove audio playback,
actual OS pointer picking or visual quality. When those behaviors change, select
the relevant typewriting/chime, click, focus or scroll interactions. Scale changes
are compatibility scope; the full journey is not required for every edit.

Retained art, font, cursor and WAV are the original repository inputs. This
migration does not make any additional licensing claim about those assets.

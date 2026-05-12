use bevy::prelude::*;

use super::super::backend::{
    character_template, move_def, resolve_turn, Action, BattlePhase, BattleState, Side,
};
use super::super::AppState;
use super::components::*;
use super::constants::*;
use super::dialogue::{BattleEventQueue, DialogueState};
use super::resources::{BattleStateRes, DisplayedCombatants, PendingMove};
use super::sequencer::Sequencer;

// ===== STARTUP =====

pub fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, CarterfightEntity));

    commands.spawn((
        Sprite::from_image(asset_server.load(CARTER_SPRITE_PATH)),
        Transform::from_xyz(CARTER_SPRITE_X, CARTER_SPRITE_Y, 0.0)
            .with_scale(Vec3::splat(CARTER_SPRITE_SCALE)),
        CarterfightEntity,
    ));

    // Health bar + label, anchored to the left edge so the fill shrinks
    // rightward as Carter takes damage.
    let half_sprite_h = CARTER_SPRITE_NATIVE_SIZE * CARTER_SPRITE_SCALE / 2.0;
    let bar_y = CARTER_SPRITE_Y - half_sprite_h - CARTER_HEALTHBAR_GAP - CARTER_HEALTHBAR_H / 2.0;
    let bar_left = CARTER_SPRITE_X - CARTER_HEALTHBAR_W / 2.0;
    let text_y =
        bar_y - CARTER_HEALTHBAR_H / 2.0 - CARTER_HEALTHBAR_TEXT_GAP - CARTER_HEALTH_TEXT_SIZE / 2.0;

    commands.spawn((
        Sprite {
            color: CARTER_HEALTHBAR_BG_COLOR,
            custom_size: Some(Vec2::new(CARTER_HEALTHBAR_W, CARTER_HEALTHBAR_H)),
            ..default()
        },
        bevy::sprite::Anchor::CENTER_LEFT,
        Transform::from_xyz(bar_left, bar_y, 0.1),
        CarterfightEntity,
    ));

    commands.spawn((
        Sprite {
            color: CARTER_HEALTHBAR_FILL_COLOR,
            custom_size: Some(Vec2::new(CARTER_HEALTHBAR_W, CARTER_HEALTHBAR_H)),
            ..default()
        },
        bevy::sprite::Anchor::CENTER_LEFT,
        Transform::from_xyz(bar_left, bar_y, 0.2),
        CarterHealthBarFill,
        CarterfightEntity,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: CARTER_HEALTH_TEXT_SIZE,
            ..default()
        },
        TextColor(CARTER_HEALTH_TEXT_COLOR),
        Transform::from_xyz(CARTER_SPRITE_X, text_y, 0.3),
        CarterHealthText,
        CarterfightEntity,
    ));

    // HUD line at the top — HP + move list. The dialogue box itself is owned
    // and rendered by `super::dialogue::DialoguePlugin`.
    commands.spawn((
        Text::new(""),
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: HUD_FONT_SIZE,
            ..default()
        },
        TextColor(HUD_TEXT),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0),
            left: Val::Px(24.0),
            ..default()
        },
        BattleHudText,
        CarterfightEntity,
    ));
}

/// Build the initial battle state and stash it as a resource. Runs once at
/// startup. The `DisplayedCombatants` mirror is seeded from the same template
/// HPs so the HUD starts in sync; the sequencer keeps them in sync from then on.
pub fn spawn_battle_state(mut commands: Commands) {
    let player = character_template("Player").expect("Player template exists");
    let opponent = character_template("Carter").expect("Carter template exists");
    let displayed = DisplayedCombatants {
        player_hp: player.current_hp,
        opponent_hp: opponent.current_hp,
    };
    let state = BattleState::new(player, opponent, BATTLE_RNG_SEED);
    commands.insert_resource(BattleStateRes(state));
    commands.insert_resource(displayed);
}

// ===== INTRO / OUTRO SCRIPTS =====

pub fn enqueue_intro_script(mut queue: ResMut<BattleEventQueue>) {
    queue.0.clear();
    // Authored by the dialogue-box PR; preserved verbatim. The third placeholder
    // line ("CARTER used SICK BEAT! ...") was dropped because the engine now
    // emits real combat narration via `BattleEvent::dialogue_text()` during play.
    queue.push_line("A wild CARTER appeared!");
    // Prompt — auto-advances after typing so `intro_to_battle_when_empty` can
    // transition without requiring an extra Space ack.
    queue.push_auto_line("What will you do?");
}

pub fn enqueue_outro_script(mut queue: ResMut<BattleEventQueue>, state: Res<BattleStateRes>) {
    queue.0.clear();
    let winner_text = match &state.0.phase {
        BattlePhase::Ended { winner } => match winner {
            Side::Player => "You beat Carter!",
            Side::Opponent => "Carter wins the fight...",
        },
        _ => "The fight is over.",
    };
    queue.push_line(winner_text);
    // Auto-advance so `outro_exit_when_empty` doesn't need a separate ack
    // press before the exit press.
    queue.push_auto_line("Press SPACE to close.");
}

// ===== INTRO/OUTRO STATE TRANSITIONS =====

/// "Dialogue is fully drained" means the queue is empty, the typewriter has
/// finished its current line, AND the sequencer isn't still gating on player
/// input. The sequencer pops the next event the moment it goes Idle, so
/// checking the queue alone would skip past the final line mid-typewriter.
fn dialogue_idle(
    queue: &BattleEventQueue,
    state: &DialogueState,
    sequencer: &Sequencer,
) -> bool {
    queue.0.is_empty() && state.is_done && sequencer.is_idle()
}

pub fn intro_to_battle_when_empty(
    queue: Res<BattleEventQueue>,
    dialogue_state: Res<DialogueState>,
    sequencer: Res<Sequencer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if dialogue_idle(&queue, &dialogue_state, &sequencer) {
        next_state.set(AppState::Battle);
    }
}

pub fn outro_exit_when_empty(
    queue: Res<BattleEventQueue>,
    dialogue_state: Res<DialogueState>,
    sequencer: Res<Sequencer>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if dialogue_idle(&queue, &dialogue_state, &sequencer)
        && keyboard.just_pressed(KeyCode::Space)
    {
        exit.write(AppExit::Success);
    }
}

// ===== BATTLE: HUD + INPUT + RESOLVE =====

pub fn update_battle_hud(
    state: Res<BattleStateRes>,
    displayed: Res<DisplayedCombatants>,
    pending: Res<PendingMove>,
    mut hud: Query<&mut Text, With<BattleHudText>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let p = &state.0.player;

    let mut s = format!(
        "{}: {}/{} HP\n\nTurn {}\n\nMoves:",
        p.name, displayed.player_hp, p.max_hp, state.0.turn_count
    );
    for (i, move_id) in p.moves.iter().enumerate() {
        let name = move_def(move_id).map(|m| m.name).unwrap_or("?");
        let marker = if pending.0 == Some(*move_id) { "→ " } else { "  " };
        s.push_str(&format!("\n{}[{}] {}", marker, i + 1, name));
    }
    match pending.0 {
        Some(move_id) => {
            let name = move_def(move_id).map(|m| m.name).unwrap_or("?");
            s.push_str(&format!(
                "\n\nUse {}? Press SPACE to confirm — or press another number to change.",
                name
            ));
        }
        None => {
            s.push_str("\n\n(Press 1-3 to choose a move.)");
        }
    }
    *text = Text::new(s);
}

/// Two-stage move flow:
///   * number keys 1/2/3 set the pending move (no commit, no engine call)
///   * SPACE confirms the pending move and runs the turn
///   * pressing another number replaces the pending move
/// All input is gated on the dialogue plugin being idle so we never step on
/// the typewriter mid-line.
pub fn battle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    dialogue_state: Res<DialogueState>,
    sequencer: Res<Sequencer>,
    mut state: ResMut<BattleStateRes>,
    mut queue: ResMut<BattleEventQueue>,
    mut pending: ResMut<PendingMove>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !dialogue_idle(&queue, &dialogue_state, &sequencer) {
        return;
    }

    if let BattlePhase::Ended { .. } = state.0.phase {
        next_state.set(AppState::OutroDialogue);
        return;
    }

    if !matches!(
        state.0.phase,
        BattlePhase::WaitingForPlayerAction | BattlePhase::Animating
    ) {
        return;
    }

    // === Selection: number key sets (or replaces) the pending move ===
    let slot = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else {
        None
    };
    if let Some(slot) = slot {
        if let Some(move_id) = state.0.player.moves.get(slot).copied() {
            pending.0 = Some(move_id);
        }
        return;
    }

    // === Confirmation: SPACE commits the pending move ===
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    let Some(player_move) = pending.0 else {
        return; // nothing selected yet — no-op
    };
    pending.0 = None;

    // v1 AI: always pick the first move. Deterministic; swap in a real picker
    // later without changing any plumbing.
    let opponent_move = match state.0.opponent.moves.first().copied() {
        Some(id) => id,
        None => return,
    };

    let events = resolve_turn(
        &mut state.0,
        Action::UseMove(player_move),
        Action::UseMove(opponent_move),
    );
    for ev in events {
        // Narration: each line waits for Space so its visual side-effect
        // (HP drop, etc.) stays in sync with the dialogue.
        queue.push(ev);
    }

    // Only step out of Animating. If the engine wrote `Ended { winner }`,
    // keep it — the existing phase-check at the top of this function fires
    // next frame (after the queue drains) and triggers the OutroDialogue
    // transition.
    if matches!(state.0.phase, BattlePhase::Animating) {
        state.0.phase = BattlePhase::WaitingForPlayerAction;
    }

    // Prompt the next turn only when the battle is still going. Auto-advance
    // so the player can pick a move without an extra ack press.
    if !matches!(state.0.phase, BattlePhase::Ended { .. }) {
        queue.push_auto_line("What will you do?");
    }
}

/// Drives the world-space health bar + label under Carter. Reads from the
/// `DisplayedCombatants` mirror, which the sequencer updates only when a
/// `Damage` event is popped — so the bar shrinks together with the matching
/// dialogue line rather than the moment `resolve_turn` runs.
pub fn update_carter_health_display(
    state: Res<BattleStateRes>,
    displayed: Res<DisplayedCombatants>,
    mut fill_q: Query<&mut Sprite, With<CarterHealthBarFill>>,
    mut text_q: Query<&mut Text2d, With<CarterHealthText>>,
) {
    let max_hp = state.0.opponent.max_hp;
    let current_hp = displayed.opponent_hp;
    let ratio = if max_hp > 0 {
        (current_hp as f32 / max_hp as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    if let Ok(mut sprite) = fill_q.single_mut() {
        sprite.custom_size = Some(Vec2::new(
            CARTER_HEALTHBAR_W * ratio,
            CARTER_HEALTHBAR_H,
        ));
    }

    if let Ok(mut text) = text_q.single_mut() {
        **text = format!("{}/{}", current_hp, max_hp);
    }
}

use bevy::prelude::*;

use super::super::backend::{
    character_template, move_def, resolve_turn, Action, BattlePhase, BattleState, Side,
};
use super::super::AppState;
use super::components::*;
use super::constants::*;
use super::dialogue::{DialogueQueue, DialogueState};
use super::resources::*;

// ===== STARTUP =====

pub fn setup_scene(mut commands: Commands) {
    commands.spawn((Camera2d, CarterfightEntity));

    // HUD line at the top — HP + move list. The dialogue box itself is owned
    // and rendered by `super::dialogue::DialoguePlugin`.
    commands.spawn((
        Text::new(""),
        TextFont {
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
/// startup.
pub fn spawn_battle_state(mut commands: Commands) {
    let player = character_template("Carter").expect("Carter template exists");
    let opponent = character_template("Rival").expect("Rival template exists");
    let state = BattleState::new(player, opponent, BATTLE_RNG_SEED);
    commands.insert_resource(BattleStateRes(state));
}

// ===== INTRO / OUTRO SCRIPTS =====

pub fn enqueue_intro_script(mut queue: ResMut<DialogueQueue>) {
    queue.0.clear();
    // Authored by the dialogue-box PR; preserved verbatim. The third placeholder
    // line ("CARTER used SICK BEAT! ...") was dropped because the engine now
    // emits real combat narration via `BattleEvent::dialogue_text()` during play.
    queue.push("A wild CARTER appeared!");
    queue.push("What will you do?");
}

pub fn enqueue_outro_script(mut queue: ResMut<DialogueQueue>, state: Res<BattleStateRes>) {
    queue.0.clear();
    let winner_text = match &state.0.phase {
        BattlePhase::Ended { winner } => match winner {
            Side::Player => "Carter wins the fight!",
            Side::Opponent => "Carter is defeated...",
        },
        _ => "The fight is over.",
    };
    queue.push(winner_text);
    queue.push("Press SPACE to close.");
}

// ===== INTRO/OUTRO STATE TRANSITIONS =====

/// "Dialogue is fully drained" means both the queue is empty AND the typewriter
/// has finished rendering its current line. Their plugin pops from the queue
/// the instant a new message is pulled into `DialogueState`, so checking the
/// queue alone would skip past the final line mid-typewriter.
fn dialogue_idle(queue: &DialogueQueue, state: &DialogueState) -> bool {
    queue.0.is_empty() && state.is_done
}

pub fn intro_to_battle_when_empty(
    queue: Res<DialogueQueue>,
    dialogue_state: Res<DialogueState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if dialogue_idle(&queue, &dialogue_state) {
        next_state.set(AppState::Battle);
    }
}

pub fn outro_exit_when_empty(
    queue: Res<DialogueQueue>,
    dialogue_state: Res<DialogueState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if dialogue_idle(&queue, &dialogue_state) && keyboard.just_pressed(KeyCode::Space) {
        exit.write(AppExit::Success);
    }
}

// ===== BATTLE: HUD + INPUT + RESOLVE =====

pub fn update_battle_hud(
    state: Res<BattleStateRes>,
    mut hud: Query<&mut Text, With<BattleHudText>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let p = &state.0.player;
    let o = &state.0.opponent;

    let mut s = format!(
        "{}: {}/{} HP    {}: {}/{} HP\n\nTurn {}\n\nMoves:",
        p.name, p.current_hp, p.max_hp, o.name, o.current_hp, o.max_hp, state.0.turn_count
    );
    for (i, move_id) in p.moves.iter().enumerate() {
        let name = move_def(move_id).map(|m| m.name).unwrap_or("?");
        s.push_str(&format!("\n  [{}] {}", i + 1, name));
    }
    *text = Text::new(s);
}

/// Reads number-key input, resolves a turn, drains events into the dialogue
/// queue as plain strings (via `BattleEvent::dialogue_text()`). Only runs
/// when the dialogue plugin is idle — otherwise the player is still reading
/// the previous turn's events.
pub fn battle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    dialogue_state: Res<DialogueState>,
    mut state: ResMut<BattleStateRes>,
    mut queue: ResMut<DialogueQueue>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !dialogue_idle(&queue, &dialogue_state) {
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

    let slot = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else {
        None
    };
    let Some(slot) = slot else { return };

    let Some(player_move) = state.0.player.moves.get(slot).copied() else {
        return;
    };

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
        queue.push(ev.dialogue_text());
    }

    // Drop the backend out of Animating once events are queued; the dialogue
    // plugin owns the player's pacing from here.
    state.0.phase = BattlePhase::WaitingForPlayerAction;
}

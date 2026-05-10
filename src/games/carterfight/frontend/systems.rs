use bevy::prelude::*;

use super::super::backend::{
    character_template, move_def, resolve_turn, Action, BattlePhase, BattleState,
};
use super::super::AppState;
use super::components::*;
use super::constants::*;
use super::resources::*;

// ===== STARTUP =====

pub fn setup_scene(mut commands: Commands) {
    commands.spawn((Camera2d, CarterfightEntity));

    // HUD line at the top — HP + move list.
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

    // Dialogue box at the bottom — placeholder. The colleague will replace
    // this with the real renderer; everything else here keeps working.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Px(20.0),
                right: Val::Px(20.0),
                height: Val::Px(140.0),
                padding: UiRect::all(Val::Px(20.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            BackgroundColor(DIALOGUE_BG),
            DialogueBoxStub,
            CarterfightEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: DIALOGUE_FONT_SIZE,
                    ..default()
                },
                TextColor(DIALOGUE_TEXT),
                DialogueLineText,
            ));
        });
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
    queue.items.clear();
    queue.push_text("Carter steps into the ring.");
    queue.push_text("His Rival cracks his knuckles.");
    queue.push_text("FIGHT!");
}

pub fn enqueue_outro_script(
    mut queue: ResMut<DialogueQueue>,
    state: Res<BattleStateRes>,
) {
    queue.items.clear();
    let winner_text = match &state.0.phase {
        BattlePhase::Ended { winner } => match winner {
            super::super::backend::Side::Player => "Carter wins the fight!",
            super::super::backend::Side::Opponent => "Carter is defeated...",
        },
        _ => "The fight is over.",
    };
    queue.push_text(winner_text);
    queue.push_text("Press SPACE to close.");
}

// ===== DIALOGUE BOX (placeholder rendering) =====

/// Re-renders the current dialogue line whenever the queue's front entry
/// changes. Uses `DialogueEntry::display_text` — the uniform string view.
pub fn update_dialogue_text(
    queue: Res<DialogueQueue>,
    mut text_query: Query<&mut Text, With<DialogueLineText>>,
) {
    if !queue.is_changed() {
        return;
    }
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };
    match queue.peek() {
        Some(entry) => *text = Text::new(entry.display_text()),
        None => *text = Text::new(""),
    }
}

/// Player advances the dialogue queue with Space (or Enter). When the queue
/// is empty during Battle, control returns to the player input system.
pub fn dialogue_advance_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut queue: ResMut<DialogueQueue>,
) {
    if queue.is_empty() {
        return;
    }
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter) {
        queue.pop();
    }
}

// ===== INTRO/OUTRO STATE TRANSITIONS =====

pub fn intro_to_battle_when_empty(
    queue: Res<DialogueQueue>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if queue.is_empty() {
        next_state.set(AppState::Battle);
    }
}

pub fn outro_exit_when_empty(
    queue: Res<DialogueQueue>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if queue.is_empty() && keyboard.just_pressed(KeyCode::Space) {
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
/// queue. Only runs when the queue is empty (otherwise the player is still
/// reading the previous turn's events) and the backend is waiting for input.
pub fn battle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BattleStateRes>,
    mut queue: ResMut<DialogueQueue>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // Wait until the player has cleared the prior turn's dialogue.
    if !queue.is_empty() {
        return;
    }

    // If the backend already declared a winner, transition out.
    if let BattlePhase::Ended { .. } = state.0.phase {
        next_state.set(AppState::OutroDialogue);
        return;
    }

    // Only listen for action input while the backend wants it.
    if !matches!(state.0.phase, BattlePhase::WaitingForPlayerAction | BattlePhase::Animating) {
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

    // v1 AI: always pick the first move. Deterministic and trivial; swap in a
    // real picker later without changing any other plumbing.
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
        queue.push_event(ev);
    }

    // Snap the backend out of Animating once the queue drains. The simplest
    // place to do it: after the queue empties, we'll be back in this system
    // and the phase check at top will let input through again.
    state.0.phase = BattlePhase::WaitingForPlayerAction;
}

//! Engine-level tests for `resolve_turn`. All pure-Rust — no Bevy `App`,
//! no window, no rendering. Each test pins a fixed seed so event sequences
//! are reproducible.

use bevy_experiments::games::carterfight::backend::{
    character_template, resolve_turn, Action, BattleEvent, BattlePhase, BattleState, Side,
};

const SEED: u64 = 42;

fn fresh_battle() -> BattleState {
    let player = character_template("Carter").expect("Carter template exists");
    let opponent = character_template("Rival").expect("Rival template exists");
    BattleState::new(player, opponent, SEED)
}

#[test]
fn damage_reduces_target_hp_and_emits_damage_event() {
    let mut state = fresh_battle();
    let starting_hp = state.opponent.current_hp;

    let events = resolve_turn(
        &mut state,
        Action::UseMove("jab"),
        Action::UseMove("jab"),
    );

    assert!(state.opponent.current_hp < starting_hp);
    assert!(state.player.current_hp < starting_hp);
    assert!(events
        .iter()
        .any(|e| matches!(e, BattleEvent::Damage { target: Side::Opponent, .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, BattleEvent::Damage { target: Side::Player, .. })));
}

#[test]
fn fatal_damage_emits_fainted_and_battle_ended() {
    let mut state = fresh_battle();

    // Drive the opponent to near-zero so the next hit kills.
    state.opponent.current_hp = 5;

    let events = resolve_turn(
        &mut state,
        Action::UseMove("haymaker"), // 20 damage
        Action::UseMove("jab"),      // would do 8, but opponent already dead
    );

    assert!(matches!(state.phase, BattlePhase::Ended { winner: Side::Player }));
    assert_eq!(state.opponent.current_hp, 0);

    let fainted_idx = events
        .iter()
        .position(|e| matches!(e, BattleEvent::Fainted { side: Side::Opponent }))
        .expect("fainted event emitted");
    let ended_idx = events
        .iter()
        .position(|e| matches!(e, BattleEvent::BattleEnded { winner: Side::Player }))
        .expect("battle ended event emitted");
    assert!(fainted_idx < ended_idx, "Fainted must precede BattleEnded");
}

#[test]
fn dead_unit_does_not_act_in_its_own_turn() {
    let mut state = fresh_battle();
    state.opponent.current_hp = 1;

    let events = resolve_turn(
        &mut state,
        Action::UseMove("jab"),      // kills opponent
        Action::UseMove("haymaker"), // opponent is dead — must be skipped
    );

    // No damage event targeting the player (opponent can't act after dying).
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, BattleEvent::Damage { target: Side::Player, .. })),
        "Dead opponent should not have acted; events: {events:#?}",
    );
    assert_eq!(state.player.current_hp, state.player.max_hp);
}

#[test]
fn unknown_move_id_is_silently_skipped() {
    let mut state = fresh_battle();
    let starting_opponent_hp = state.opponent.current_hp;

    let events = resolve_turn(
        &mut state,
        Action::UseMove("nonexistent_move"),
        Action::UseMove("jab"),
    );

    // Player's bogus move produced no UseMove/Damage events at all.
    assert!(
        !events.iter().any(|e| matches!(e, BattleEvent::UseMove { side: Side::Player, .. })),
        "Unknown move should not emit a UseMove event",
    );
    assert_eq!(state.opponent.current_hp, starting_opponent_hp);

    // Opponent's jab still happened.
    assert!(state.player.current_hp < state.player.max_hp);
}

#[test]
fn turn_counter_advances_each_turn() {
    let mut state = fresh_battle();
    assert_eq!(state.turn_count, 0);

    let _ = resolve_turn(&mut state, Action::UseMove("jab"), Action::UseMove("jab"));
    assert_eq!(state.turn_count, 1);

    let _ = resolve_turn(&mut state, Action::UseMove("jab"), Action::UseMove("jab"));
    assert_eq!(state.turn_count, 2);
}

#[test]
fn deterministic_under_fixed_seed() {
    let actions_a = [
        (Action::UseMove("haymaker"), Action::UseMove("jab")),
        (Action::UseMove("jab"), Action::UseMove("haymaker")),
    ];
    let actions_b = actions_a.clone();

    let mut state_a = fresh_battle();
    let mut events_a = Vec::new();
    for (p, o) in actions_a {
        events_a.extend(resolve_turn(&mut state_a, p, o));
    }

    let mut state_b = fresh_battle();
    let mut events_b = Vec::new();
    for (p, o) in actions_b {
        events_b.extend(resolve_turn(&mut state_b, p, o));
    }

    // Same seed + same actions ⇒ identical HP, phase, turn count, event list.
    assert_eq!(state_a.player.current_hp, state_b.player.current_hp);
    assert_eq!(state_a.opponent.current_hp, state_b.opponent.current_hp);
    assert_eq!(state_a.turn_count, state_b.turn_count);
    assert_eq!(state_a.phase, state_b.phase);
    assert_eq!(events_a.len(), events_b.len());
}

#[test]
fn every_event_has_non_empty_dialogue_text() {
    let mut state = fresh_battle();
    state.opponent.current_hp = 3;

    let events = resolve_turn(
        &mut state,
        Action::UseMove("haymaker"),
        Action::UseMove("jab"),
    );

    for event in &events {
        let text = event.dialogue_text();
        assert!(!text.is_empty(), "event {event:?} produced empty dialogue text");
    }
}

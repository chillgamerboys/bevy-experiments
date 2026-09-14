//! Initial outcomes must be available to the encounter archive, not inferred later.
use super::*;
use crate::{
    catalog::ContentCatalog,
    scenario::{Scenario, StartingStatus, StockScenario},
};

#[test]
fn constructor_returns_initial_round_turn_and_status_outcomes_without_changing_combat() {
    let catalog = ContentCatalog::builtin().expect("catalog");
    let mut scenario = Scenario::stock(StockScenario::Prototype, 42, &catalog).expect("scenario");
    for actor in scenario.heroes.iter_mut().chain(&mut scenario.enemies) {
        actor.starting_statuses.push(StartingStatus {
            kind: StatusKind::Bleed,
            source: None,
            remaining: None,
        });
    }
    let expected = Combat::from_scenario(&catalog, &scenario)
        .expect("ordinary construction")
        .snapshot();
    let (mut combat, events) =
        Combat::from_scenario_with_events(&catalog, &scenario).expect("construction");
    assert_eq!(combat.snapshot(), expected);
    assert!(matches!(
        events.first().expect("initial round").kind,
        CombatEventKind::RoundStarted { round: 1 }
    ));
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, CombatEventKind::TurnStarted { .. })));
    assert!(events.iter().any(|event| matches!(
        event.kind,
        CombatEventKind::StatusTriggered {
            kind: StatusKind::Bleed,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event.kind,
        CombatEventKind::Damage {
            kind: DamageKind::Bleed,
            ..
        }
    )));
    assert!(events.array_windows::<2>().all(|[a, b]| a.id + 1 == b.id));
    let actor = expected.active_actor.expect("decision");
    let next = combat.apply(actor, CombatAction::Wait).expect("wait");
    assert_eq!(
        next.first().expect("action event").id,
        events.last().expect("initial event").id + 1
    );
}

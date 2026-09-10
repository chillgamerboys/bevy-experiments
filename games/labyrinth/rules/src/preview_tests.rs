//! Forecast evidence: immediate effects agree with commits, never future boundaries.

use super::*;
use crate::{ActionPreview, PreviewEvent, PreviewStatus, SkillId, DEFAULT_HERO_ROSTER};

fn fixture(source: ActorId) -> Combat {
    let mut combat = Combat::new(42, DEFAULT_HERO_ROSTER).expect("valid setup");
    for _ in 0..MAX_ACTORS * 2 {
        if combat.state.active_actor == Some(source) {
            return combat;
        }
        let active = combat.state.active_actor.expect("live combat");
        combat
            .apply(active, CombatAction::Wait)
            .expect("advance without damage");
    }
    assert_eq!(combat.state.active_actor, Some(source));
    combat
}

fn attach(combat: &mut Combat, source: ActorId, target: ActorId, kind: StatusKind) {
    combat
        .add_status(source, target, kind, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("fixture status");
}

fn compare(combat: &Combat, source: ActorId, action: CombatAction) -> ActionPreview {
    let original = combat.clone();
    let snapshot = combat.snapshot();
    let preview = snapshot.preview_action(source, &action).expect("preview");
    assert_eq!(
        snapshot.preview_action(source, &action),
        Ok(preview.clone()),
        "repeatable forecast"
    );
    assert_eq!(snapshot, combat.snapshot());
    assert_eq!(
        *combat, original,
        "no RNG, status/event counter, cursor or state mutation"
    );

    let mut immediate = combat.clone();
    let mut events = Vec::new();
    immediate
        .resolver()
        .resolve_immediate(source, action, &mut events, &mut Work(MAX_WORK))
        .expect("immediate resolution");
    assert_eq!(immediate.rng, combat.rng);
    assert_eq!(immediate.cursor, combat.cursor);
    assert_eq!(immediate.state.turn_id, snapshot.turn_id);
    assert_eq!(
        immediate.state.boundary_sequence,
        snapshot.boundary_sequence
    );
    assert_eq!(immediate.state.initiative, snapshot.initiative);
    for projected in &preview.actors {
        let actor = immediate
            .state
            .actor(projected.actor)
            .expect("actor retained");
        assert_eq!(projected.after.hp, actor.hp);
        assert_eq!(projected.after.rank, immediate.state.rank(actor.id));
        assert_eq!(
            projected.after.statuses,
            actor
                .statuses
                .iter()
                .map(PreviewStatus::from)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(preview.outcome, immediate.state.outcome);
    let projected_events: Vec<_> = events
        .iter()
        .filter(|event| !matches!(event.kind, CombatEventKind::Action { .. }))
        .map(|event| PreviewEvent::try_from(event.kind.clone()).expect("immediate event"))
        .collect();
    assert_eq!(preview.events, projected_events);

    let mut committed = combat.clone();
    let committed_events = committed
        .apply(source, action)
        .expect("current-turn commit");
    assert_eq!(
        committed_events
            .iter()
            .take(events.len())
            .cloned()
            .collect::<Vec<_>>(),
        events,
        "live execution uses exactly the same immediate prefix before turn advancement"
    );
    preview
}

#[test]
fn off_turn_inspection_preserves_current_turn_commit_authority() {
    let combat = fixture(ActorId(1));
    let snapshot = combat.snapshot();
    let action = CombatAction::Skill {
        skill: SkillId::BackRankShot,
        target: ActorId(106),
    };
    assert_eq!(
        snapshot.validate_action(ActorId(4), &action),
        Err(RuleError::WrongActor)
    );
    assert!(snapshot.validate_action_target(ActorId(4), &action).is_ok());
    let preview = snapshot
        .preview_action(ActorId(4), &action)
        .expect("off-turn forecast");
    assert_eq!(preview.damage.first().expect("hit").base, 7);
    assert_eq!(snapshot, combat.snapshot());
    let invalid = CombatAction::Skill {
        skill: SkillId::BackRankShot,
        target: ActorId(101),
    };
    assert_eq!(
        snapshot.preview_action(ActorId(4), &invalid),
        Err(RuleError::WrongTargetRank)
    );
    assert_eq!(
        snapshot.preview_action(ActorId(1), &action),
        Err(RuleError::UnknownSkill)
    );
}

#[test]
fn direct_hit_separates_base_modifiers_and_remaining_hp_cap() {
    let mut combat = fixture(ActorId(1));
    attach(&mut combat, ActorId(101), ActorId(1), StatusKind::Weakened);
    attach(&mut combat, ActorId(101), ActorId(101), StatusKind::Brace);
    combat.actor_mut(ActorId(101)).expect("enemy").hp = 2;
    let preview = compare(
        &combat,
        ActorId(1),
        CombatAction::Skill {
            skill: SkillId::FrontStrike,
            target: ActorId(101),
        },
    );
    let hit = preview.damage.first().expect("hit");
    assert_eq!((hit.base, hit.effective, hit.hp_loss), (7, 3, 2));
    assert_eq!(preview.actor(ActorId(101)).expect("target").after.hp, 0);
    assert_eq!(
        preview.actor(ActorId(101)).expect("target").after.rank,
        None
    );
    assert_eq!(
        preview.actor(ActorId(102)).expect("compacted").after.rank,
        Some(1)
    );
}

#[test]
fn lethal_hits_suppress_status_and_movement_followups() {
    for (source, skill, hp) in [
        (ActorId(1), SkillId::DrivingBlow, 4),
        (ActorId(2), SkillId::BleedingCut, 3),
    ] {
        let mut combat = fixture(source);
        combat.actor_mut(ActorId(101)).expect("enemy").hp = hp;
        let preview = compare(
            &combat,
            source,
            CombatAction::Skill {
                skill,
                target: ActorId(101),
            },
        );
        let target = preview.actor(ActorId(101)).expect("target");
        assert_eq!(target.after.hp, 0);
        assert!(target.after.statuses.is_empty());
        assert!(!preview.events.iter().any(|event| matches!(
            event,
            PreviewEvent::StatusApplied(_)
                | PreviewEvent::Moved {
                    actor: ActorId(101),
                    ..
                }
        )));
    }
}

#[test]
fn healing_and_rescue_use_the_committed_caps_and_rounding() {
    let mut combat = fixture(ActorId(5));
    combat.actor_mut(ActorId(1)).expect("hero").hp = 31;
    let preview = compare(
        &combat,
        ActorId(5),
        CombatAction::Skill {
            skill: SkillId::Mend,
            target: ActorId(1),
        },
    );
    assert_eq!(preview.actor(ActorId(1)).expect("hero").after.hp, 32);
    assert!(preview.events.contains(&PreviewEvent::Healed {
        source: ActorId(5),
        target: ActorId(1),
        amount: 1
    }));
    combat.actor_mut(ActorId(4)).expect("scout").hp = 0;
    let preview = compare(
        &combat,
        ActorId(5),
        CombatAction::Rescue { ally: ActorId(4) },
    );
    assert_eq!(preview.actor(ActorId(4)).expect("scout").after.hp, 6);
    assert_eq!(
        preview.actor(ActorId(4)).expect("scout").after.rank,
        Some(4)
    );
}

#[test]
fn status_application_refresh_cleanse_and_movement_share_resolution() {
    let mut combat = fixture(ActorId(2));
    let action = CombatAction::Skill {
        skill: SkillId::BleedingCut,
        target: ActorId(101),
    };
    let applied = compare(&combat, ActorId(2), action);
    assert!(applied.events.iter().any(
        |event| matches!(event, PreviewEvent::StatusApplied(status) if status.remaining == 3)
    ));
    // The real counter may remember many removed effects not present in a snapshot.
    combat.next_status = 1000;
    attach(&mut combat, ActorId(3), ActorId(101), StatusKind::Bleed);
    combat
        .actor_mut(ActorId(101))
        .expect("enemy")
        .statuses
        .first_mut()
        .expect("bleed")
        .remaining = 1;
    let refreshed = compare(&combat, ActorId(2), action);
    assert_eq!(
        refreshed
            .actor(ActorId(101))
            .expect("enemy")
            .after
            .statuses
            .len(),
        1
    );
    assert!(refreshed.events.iter().any(|event| matches!(event,
        PreviewEvent::StatusRefreshed(status) if status.source == ActorId(2) && status.remaining == 3)));
    attach(&mut combat, ActorId(103), ActorId(2), StatusKind::Bleed);
    let cleansed = compare(
        &combat,
        ActorId(2),
        CombatAction::Skill {
            skill: SkillId::CleanBlade,
            target: ActorId(2),
        },
    );
    assert!(cleansed
        .actor(ActorId(2))
        .expect("hero")
        .after
        .statuses
        .is_empty());
    let moved = compare(
        &combat,
        ActorId(2),
        CombatAction::Reposition { ally: ActorId(3) },
    );
    assert_eq!(moved.actor(ActorId(2)).expect("hero").after.rank, Some(3));
    assert_eq!(moved.actor(ActorId(3)).expect("ally").after.rank, Some(2));
    assert_eq!(
        moved.actor(ActorId(2)).expect("hero").after.statuses,
        moved.actor(ActorId(2)).expect("hero").before.statuses
    );
}

#[test]
fn forecast_never_predicts_future_bleed_ticks_or_the_next_round_roll() {
    let mut combat = fixture(ActorId(1));
    attach(&mut combat, ActorId(2), ActorId(101), StatusKind::Bleed);
    let preview = compare(&combat, ActorId(1), CombatAction::Wait);
    assert!(preview.damage.is_empty());
    assert!(preview.events.is_empty());
    assert!(preview
        .actors
        .iter()
        .all(|actor| actor.before == actor.after));
    for _ in 0..MAX_ACTORS {
        if combat.cursor + 1 == combat.state.initiative.len() {
            break;
        }
        let active = combat.state.active_actor.expect("live turn");
        combat
            .apply(active, CombatAction::Wait)
            .expect("advance to round end");
    }
    let source = combat.state.active_actor.expect("last round actor");
    let before = combat.clone();
    compare(&combat, source, CombatAction::Wait);
    assert_eq!(combat, before);
    let events = combat
        .apply(source, CombatAction::Wait)
        .expect("commit advances round");
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, CombatEventKind::RoundStarted { .. })));
}

#[test]
fn periodic_damage_uses_the_same_calculation_without_direct_modifiers() {
    let mut combat = fixture(ActorId(1));
    attach(&mut combat, ActorId(101), ActorId(1), StatusKind::Brace);
    attach(
        &mut combat,
        ActorId(101),
        ActorId(101),
        StatusKind::Weakened,
    );
    let mut damage = Vec::new();
    let mut events = Vec::new();
    let mut resolver = combat.resolver();
    resolver.damage = Some(&mut damage);
    resolver
        .effect(
            ActorId(101),
            ActorId(1),
            Effect::StatusDamage(DamageKind::Bleed),
            2,
            &mut events,
            &mut Work(MAX_WORK),
        )
        .expect("shared periodic calculation");
    let hit = damage.first().expect("periodic hit");
    assert_eq!((hit.base, hit.effective, hit.hp_loss), (2, 2, 2));
    assert_eq!(hit.kind, DamageKind::Bleed);
}

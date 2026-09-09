use super::*;

#[test]
fn rejects_duplicate_heroes_and_round_trips_public_state() {
    assert_eq!(
        Combat::new(42, [HeroClass::Scout; 4]),
        Err(RuleError::DuplicateHero)
    );
    let combat = Combat::new(42, HeroClass::ALL).expect("valid party");
    let encoded = serde_json::to_string(&combat.snapshot()).expect("serialize");
    let decoded: CombatSnapshot = serde_json::from_str(&encoded).expect("validate snapshot");
    assert_eq!(decoded, combat.snapshot());
    assert!(!encoded.contains("rng"));
}

#[test]
fn shuffled_hero_selection_preserves_seat_ids_but_starts_in_role_ranks() {
    let combat = Combat::new(
        42,
        [
            HeroClass::FieldMedic,
            HeroClass::Scout,
            HeroClass::Gatekeeper,
            HeroClass::Knifehand,
        ],
    )
    .expect("shuffled valid party");
    assert_eq!(
        combat.state.actor(ActorId(1)).expect("host hero").kind,
        ActorKind::Hero(HeroClass::FieldMedic)
    );
    assert_eq!(
        combat.state.hero_formation,
        [ActorId(3), ActorId(4), ActorId(2), ActorId(1)]
    );
    assert_eq!(combat.state.rank(ActorId(1)), Some(4));
    combat.snapshot().validate().expect("ownership is not rank");
}

#[test]
fn canonical_content_fingerprint_is_pinned_and_not_just_package_version() {
    assert_eq!(
        crate::rules_fingerprint(),
        "cc5066f78bba51e75b56d779b248c66b87532faa9bbe279f46ee493fedeef30b"
    );
    assert_eq!(crate::rules_fingerprint(), crate::rules_fingerprint());
}

fn fixture() -> Combat {
    Combat::new(42, HeroClass::ALL).expect("valid party")
}

fn attach(
    combat: &mut Combat,
    source: ActorId,
    bearer: ActorId,
    kind: StatusKind,
) -> Vec<CombatEvent> {
    let mut events = Vec::new();
    combat
        .add_status(source, bearer, kind, &mut events, &mut Work(MAX_WORK))
        .expect("valid fixture status");
    events
}

fn boundary(combat: &mut Combat, timing: Boundary, owner: Option<ActorId>) -> Vec<CombatEvent> {
    let mut events = Vec::new();
    combat
        .boundary(timing, owner, &mut events, &mut Work(MAX_WORK))
        .expect("bounded fixture boundary");
    events
}

fn wait_for(combat: &mut Combat, target: ActorId) {
    for _ in 0..32 {
        if combat.state.active_actor == Some(target) {
            return;
        }
        let active = combat.state.active_actor.expect("fixture not terminal");
        combat
            .apply(active, CombatAction::Wait)
            .expect("wait legal");
    }
    assert_eq!(
        combat.state.active_actor,
        Some(target),
        "bounded wait for actor"
    );
}

fn status(combat: &Combat, actor: ActorId, kind: StatusKind) -> Option<&StatusInstance> {
    combat
        .state
        .actor(actor)
        .expect("fixture actor")
        .statuses
        .iter()
        .find(|instance| instance.kind == kind)
}

fn damage_fixture(combat: &mut Combat, actor: ActorId, hp: u16) {
    let current = combat.state.actor(actor).expect("fixture actor").hp;
    combat
        .damage(
            ActorId(101),
            actor,
            current.saturating_sub(hp),
            DamageKind::Direct,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("fixture damage");
}

#[test]
fn frozen_seeded_initiative_rerolls_and_breaks_ties_deterministically() {
    let mut combat = fixture();
    let initial = combat.state.initiative.clone();
    assert_eq!(initial, fixture().state.initiative);
    assert_ne!(
        initial,
        Combat::new(43, HeroClass::ALL)
            .expect("party")
            .state
            .initiative
    );
    let mut acted = std::collections::BTreeSet::new();
    while combat.state.round == 1 {
        let active = combat.state.active_actor.expect("active");
        assert!(acted.insert(active));
        combat.apply(active, CombatAction::Wait).expect("wait");
    }
    assert_eq!(acted.len(), 8);
    assert_eq!(combat.state.round, 2);
    assert_ne!(initial, combat.state.initiative);
    for seed in 0..64 {
        let candidate = Combat::new(seed, HeroClass::ALL).expect("party");
        candidate.state.validate().expect("sorted unique tiebreaks");
        assert_eq!(
            candidate,
            Combat::new(seed, HeroClass::ALL).expect("same seed")
        );
    }
}

#[test]
fn rejected_actions_leave_state_rng_and_counters_unchanged() {
    let mut combat = fixture();
    let previous = combat.clone();
    assert_eq!(
        combat.apply(ActorId(999), CombatAction::Wait),
        Err(RuleError::WrongActor)
    );
    assert_eq!(combat, previous);
    let active = combat.state.active_actor.expect("active");
    assert!(combat
        .apply(
            active,
            CombatAction::Skill {
                skill: crate::SkillId::FrontStrike,
                target: ActorId(999)
            }
        )
        .is_err());
    assert_eq!(combat, previous);
    assert_eq!(
        combat.apply_with_budget(active, CombatAction::Defend, 1),
        Err(RuleError::WorkLimit)
    );
    assert_eq!(combat, previous, "partial effects/event IDs never commit");
}

#[test]
fn overflow_during_round_roll_does_not_consume_rng_or_partial_state() {
    let mut combat = fixture();
    while combat.cursor < combat.state.initiative.len() - 1 {
        let active = combat.state.active_actor.expect("active");
        combat.apply(active, CombatAction::Wait).expect("wait");
    }
    let previous = combat.clone();
    let active = combat.state.active_actor.expect("last actor");
    assert_eq!(
        combat.apply_with_budget(active, CombatAction::Wait, 6),
        Err(RuleError::WorkLimit)
    );
    assert_eq!(combat, previous);
}

#[test]
fn all_authored_heroes_have_four_skills_and_displacement_has_legal_fallbacks() {
    for class in HeroClass::ALL {
        assert_eq!(class.skills().len(), 4);
        for skill in class.skills() {
            let definition = skill_definition(*skill);
            assert!(definition.source_ranks > 0 && definition.source_ranks < 16);
            assert!(definition.target_ranks > 0 && definition.target_ranks < 16);
            assert!(!definition.effects.is_empty());
        }
    }
    let mut combat = fixture();
    wait_for(&mut combat, ActorId(1));
    combat.state.hero_formation.reverse();
    let actions = combat.legal_actions(ActorId(1));
    assert!(actions.contains(&CombatAction::Wait));
    assert!(actions.contains(&CombatAction::Defend));
    assert_eq!(
        combat.validate_action(
            ActorId(1),
            &CombatAction::Skill {
                skill: crate::SkillId::FrontStrike,
                target: ActorId(101)
            }
        ),
        Err(RuleError::WrongRank)
    );
}

#[test]
fn reposition_preserves_actor_status_and_frozen_initiative() {
    let mut combat = fixture();
    wait_for(&mut combat, ActorId(2));
    attach(&mut combat, ActorId(103), ActorId(2), StatusKind::Bleed);
    let before_order: Vec<_> = combat
        .state
        .initiative
        .iter()
        .map(|entry| entry.actor)
        .collect();
    let id = status(&combat, ActorId(2), StatusKind::Bleed)
        .expect("bleed")
        .id;
    combat
        .apply(ActorId(2), CombatAction::Reposition { ally: ActorId(1) })
        .expect("adjacent swap");
    assert_eq!(combat.state.rank(ActorId(2)), Some(1));
    assert_eq!(combat.state.rank(ActorId(1)), Some(2));
    assert_eq!(
        status(&combat, ActorId(2), StatusKind::Bleed)
            .expect("same bearer")
            .id,
        id
    );
    assert_eq!(
        before_order,
        combat
            .state
            .initiative
            .iter()
            .map(|entry| entry.actor)
            .collect::<Vec<_>>()
    );
}

#[test]
fn bleed_ticks_exactly_three_future_starts_and_normal_healing_does_not_cleanse() {
    let mut combat = fixture();
    let hero = ActorId(1);
    let hp = combat.state.actor(hero).expect("hero").hp;
    attach(&mut combat, ActorId(103), hero, StatusKind::Bleed);
    assert_eq!(
        combat.state.actor(hero).expect("hero").hp,
        hp,
        "application never ticks"
    );
    boundary(&mut combat, Boundary::OwnerTurnEnd, Some(hero));
    assert_eq!(
        status(&combat, hero, StatusKind::Bleed)
            .expect("bleed")
            .remaining,
        3
    );
    for remaining in [2, 1, 0] {
        let events = boundary(&mut combat, Boundary::OwnerTurnStart, Some(hero));
        assert!(events.iter().any(|event| matches!(
            event.kind,
            CombatEventKind::Damage {
                amount: 2,
                kind: DamageKind::Bleed,
                ..
            }
        )));
        assert_eq!(
            status(&combat, hero, StatusKind::Bleed).map_or(0, |instance| instance.remaining),
            remaining
        );
    }
    assert_eq!(combat.state.actor(hero).expect("hero").hp, hp - 6);
    attach(&mut combat, ActorId(103), hero, StatusKind::Bleed);
    combat
        .effect(
            ActorId(4),
            hero,
            Effect::Heal(8),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("heal");
    assert!(status(&combat, hero, StatusKind::Bleed).is_some());
    let events = combat.cleanse(
        hero,
        StatusTag::Bleeding,
        &mut Vec::new(),
        &mut Work(MAX_WORK),
    );
    assert!(events.is_ok());
    assert!(status(&combat, hero, StatusKind::Bleed).is_none());
}

#[test]
fn bleed_refresh_uses_single_instance_latest_source_and_no_hidden_stack() {
    let mut combat = fixture();
    let hero = ActorId(1);
    attach(&mut combat, ActorId(103), hero, StatusKind::Bleed);
    boundary(&mut combat, Boundary::OwnerTurnStart, Some(hero));
    let previous = status(&combat, hero, StatusKind::Bleed)
        .expect("bleed")
        .clone();
    let events = attach(&mut combat, ActorId(2), hero, StatusKind::Bleed);
    let refreshed = status(&combat, hero, StatusKind::Bleed).expect("bleed");
    assert_eq!(refreshed.id, previous.id);
    assert_eq!(refreshed.potency, 2);
    assert_eq!(refreshed.remaining, 3);
    assert_eq!(refreshed.source, ActorId(2));
    assert_eq!(combat.state.actor(hero).expect("hero").statuses.len(), 1);
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, CombatEventKind::StatusRefreshed { .. })));
}

#[test]
fn source_death_does_not_remove_bleed_from_its_bearer() {
    let mut combat = fixture();
    attach(&mut combat, ActorId(103), ActorId(1), StatusKind::Bleed);
    damage_fixture(&mut combat, ActorId(103), 0);
    assert!(!combat.state.enemy_formation.contains(&ActorId(103)));
    let hp = combat.state.actor(ActorId(1)).expect("hero").hp;
    boundary(&mut combat, Boundary::OwnerTurnStart, Some(ActorId(1)));
    assert_eq!(combat.state.actor(ActorId(1)).expect("hero").hp, hp - 2);
}

#[test]
fn lethal_bleed_consumes_slot_preserves_formation_and_clears_only_requested_statuses() {
    let mut combat = fixture();
    let target = combat.state.initiative.get(1).expect("second entry").actor;
    // Choose a hero's pending slot; never alter the already committed active phase.
    let target = if target.0 < 100 {
        target
    } else {
        combat
            .state
            .initiative
            .iter()
            .skip(1)
            .find(|entry| entry.actor.0 < 100)
            .expect("pending hero")
            .actor
    };
    damage_fixture(&mut combat, target, 2);
    attach(&mut combat, ActorId(103), target, StatusKind::Bleed);
    attach(&mut combat, ActorId(4), target, StatusKind::Haste);
    let original_formation = combat.state.hero_formation.clone();
    let mut observed = Vec::new();
    for _ in 0..8 {
        if !combat.state.actor(target).expect("hero").standing() {
            break;
        }
        let active = combat.state.active_actor.expect("active");
        observed.extend(combat.apply(active, CombatAction::Wait).expect("wait"));
    }
    assert!(!combat.state.actor(target).expect("hero").standing());
    assert_ne!(combat.state.active_actor, Some(target));
    assert!(
        combat
            .state
            .initiative
            .iter()
            .find(|entry| entry.actor == target)
            .expect("entry")
            .completed
    );
    assert_eq!(combat.state.hero_formation, original_formation);
    assert!(status(&combat, target, StatusKind::Bleed).is_none());
    assert!(status(&combat, target, StatusKind::Haste).is_some());
    assert!(observed.iter().any(
        |event| matches!(event.kind, CombatEventKind::TurnSkipped { actor } if actor == target)
    ));
}

#[test]
fn brace_reduces_only_direct_damage_and_expires_before_next_action() {
    let mut combat = fixture();
    let hero = ActorId(1);
    attach(&mut combat, hero, hero, StatusKind::Brace);
    let hp = combat.state.actor(hero).expect("hero").hp;
    combat
        .effect(
            ActorId(101),
            hero,
            Effect::Damage(5),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("hit");
    assert_eq!(combat.state.actor(hero).expect("hero").hp, hp - 3);
    attach(&mut combat, ActorId(103), hero, StatusKind::Bleed);
    boundary(&mut combat, Boundary::OwnerTurnStart, Some(hero));
    assert_eq!(combat.state.actor(hero).expect("hero").hp, hp - 5);
    assert!(status(&combat, hero, StatusKind::Brace).is_none());
    combat
        .effect(
            ActorId(101),
            hero,
            Effect::Damage(5),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("unbraced hit");
    assert_eq!(combat.state.actor(hero).expect("hero").hp, hp - 10);
}

#[test]
fn status_boundary_priority_and_activation_are_explicit() {
    let mut combat = fixture();
    let hero = ActorId(1);
    attach(&mut combat, ActorId(103), hero, StatusKind::Bleed);
    attach(&mut combat, hero, hero, StatusKind::Brace);
    let events = boundary(&mut combat, Boundary::OwnerTurnStart, Some(hero));
    let brace_removed = events
        .iter()
        .position(|event| {
            matches!(
                event.kind,
                CombatEventKind::StatusRemoved {
                    kind: StatusKind::Brace,
                    ..
                }
            )
        })
        .expect("brace expiry");
    let bleed_triggered = events
        .iter()
        .position(|event| {
            matches!(
                event.kind,
                CombatEventKind::StatusTriggered {
                    kind: StatusKind::Bleed,
                    ..
                }
            )
        })
        .expect("bleed trigger");
    assert!(
        brace_removed < bleed_triggered,
        "priority wins over earlier bleed instance ID"
    );

    // Model an effect created during the next boundary: eligibility prevents that
    // phase from triggering/decrementing it, even if the kind matches its timing.
    let next_eligible = combat.state.boundary_sequence + 2;
    combat
        .actor_mut(hero)
        .expect("hero")
        .statuses
        .iter_mut()
        .find(|status| status.kind == StatusKind::Bleed)
        .expect("bleed")
        .eligible_boundary = next_eligible;
    let hp = combat.state.actor(hero).expect("hero").hp;
    boundary(&mut combat, Boundary::OwnerTurnStart, Some(hero));
    assert_eq!(combat.state.actor(hero).expect("hero").hp, hp);
    assert_eq!(
        status(&combat, hero, StatusKind::Bleed)
            .expect("bleed")
            .remaining,
        2
    );
    boundary(&mut combat, Boundary::OwnerTurnStart, Some(hero));
    assert_eq!(combat.state.actor(hero).expect("hero").hp, hp - 2);
}

#[test]
fn haste_and_weakened_derive_stats_and_restore_base_without_order_drift() {
    let mut combat = fixture();
    let hero = ActorId(1);
    let initial = combat.state.initiative.clone();
    attach(&mut combat, ActorId(4), hero, StatusKind::Haste);
    attach(&mut combat, ActorId(103), hero, StatusKind::Weakened);
    assert_eq!(combat.state.actor(hero).expect("hero").speed(), 5);
    assert_eq!(
        combat
            .state
            .actor(hero)
            .expect("hero")
            .modifier(Stat::OutgoingDamage),
        -2
    );
    assert_eq!(combat.state.initiative, initial);
    let enemy_hp = combat.state.actor(ActorId(101)).expect("enemy").hp;
    combat
        .effect(
            hero,
            ActorId(101),
            Effect::Damage(1),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("clamped weak hit");
    assert_eq!(
        combat.state.actor(ActorId(101)).expect("enemy").hp,
        enemy_hp
    );
    boundary(&mut combat, Boundary::OwnerTurnEnd, Some(hero));
    assert!(status(&combat, hero, StatusKind::Weakened).is_some());
    boundary(&mut combat, Boundary::OwnerTurnEnd, Some(hero));
    assert_eq!(
        combat
            .state
            .actor(hero)
            .expect("hero")
            .modifier(Stat::OutgoingDamage),
        0
    );
    boundary(&mut combat, Boundary::RoundEnd, None);
    assert_eq!(combat.state.actor(hero).expect("hero").speed(), 5);
    boundary(&mut combat, Boundary::RoundEnd, None);
    assert_eq!(combat.state.actor(hero).expect("hero").speed(), 2);
    assert_eq!(combat.state.actor(hero).expect("hero").base_speed, 2);
}

#[test]
fn speed_buff_affects_next_round_roll_only() {
    let mut combat = fixture();
    let hero = ActorId(1);
    attach(&mut combat, ActorId(4), hero, StatusKind::Haste);
    assert_eq!(
        combat
            .state
            .initiative
            .iter()
            .find(|entry| entry.actor == hero)
            .expect("entry")
            .speed,
        2
    );
    while combat.state.round == 1 {
        let active = combat.state.active_actor.expect("active");
        combat.apply(active, CombatAction::Wait).expect("wait");
    }
    assert_eq!(
        combat
            .state
            .initiative
            .iter()
            .find(|entry| entry.actor == hero)
            .expect("entry")
            .speed,
        5
    );
}

#[test]
fn rescue_before_pending_slot_can_act_but_after_consumption_cannot_act_twice() {
    let mut combat = fixture();
    let pending = combat
        .state
        .initiative
        .iter()
        .skip(combat.cursor + 1)
        .find(|entry| entry.actor.0 < 100)
        .expect("pending hero")
        .actor;
    damage_fixture(&mut combat, pending, 0);
    combat
        .effect(
            ActorId(4),
            pending,
            Effect::Rescue(25),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("rescue");
    let restored = combat.state.actor(pending).expect("hero");
    assert_eq!(restored.hp, restored.max_hp.div_ceil(4));
    wait_for(&mut combat, pending);
    combat
        .apply(pending, CombatAction::Wait)
        .expect("uses its pending slot");
    let round = combat.state.round;
    damage_fixture(&mut combat, pending, 0);
    combat
        .effect(
            ActorId(4),
            pending,
            Effect::Rescue(25),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("rescue again");
    while combat.state.round == round {
        let active = combat.state.active_actor.expect("active");
        assert_ne!(active, pending);
        combat.apply(active, CombatAction::Wait).expect("wait");
    }
}

#[test]
fn rescue_of_actor_absent_from_round_never_inserts_a_bonus_entry() {
    let mut combat = fixture();
    damage_fixture(&mut combat, ActorId(1), 0);
    // Start an explicit fixture round with this hero excluded.
    combat
        .start_round(&mut Vec::new(), &mut Work(MAX_WORK))
        .expect("next fixture round");
    combat
        .seek_decision(&mut Vec::new(), &mut Work(MAX_WORK))
        .expect("decision");
    assert!(!combat
        .state
        .initiative
        .iter()
        .any(|entry| entry.actor == ActorId(1)));
    combat
        .effect(
            ActorId(4),
            ActorId(1),
            Effect::Rescue(25),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("rescue");
    let round = combat.state.round;
    while combat.state.round == round {
        let active = combat.state.active_actor.expect("active");
        assert_ne!(active, ActorId(1));
        combat.apply(active, CombatAction::Wait).expect("wait");
    }
    assert!(combat
        .state
        .initiative
        .iter()
        .any(|entry| entry.actor == ActorId(1)));
}

#[test]
fn killed_enemy_compacts_without_moving_statuses_or_resolving_later_skill_effects() {
    let mut combat = fixture();
    wait_for(&mut combat, ActorId(1));
    attach(&mut combat, ActorId(2), ActorId(102), StatusKind::Bleed);
    let status_id = status(&combat, ActorId(102), StatusKind::Bleed)
        .expect("bleed")
        .id;
    damage_fixture(&mut combat, ActorId(101), 4);
    combat
        .apply(
            ActorId(1),
            CombatAction::Skill {
                skill: crate::SkillId::DrivingBlow,
                target: ActorId(101),
            },
        )
        .expect("kill before displacement");
    assert_eq!(
        combat.state.enemy_formation,
        [ActorId(102), ActorId(103), ActorId(104)]
    );
    assert_eq!(
        status(&combat, ActorId(102), StatusKind::Bleed)
            .expect("status stayed on actor")
            .id,
        status_id
    );
    assert_eq!(combat.state.rank(ActorId(101)), None);
}

#[test]
fn push_and_pull_shift_ranks_without_crossing_boundaries() {
    let mut combat = fixture();
    combat
        .move_actor(ActorId(101), 1, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("push");
    assert_eq!(
        combat.state.enemy_formation,
        [ActorId(102), ActorId(101), ActorId(103), ActorId(104)]
    );
    combat
        .move_actor(ActorId(104), 1, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("boundary no-op");
    assert_eq!(combat.state.rank(ActorId(104)), Some(4));
    combat
        .move_actor(ActorId(104), -1, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("pull");
    assert_eq!(combat.state.rank(ActorId(104)), Some(3));
}

#[test]
fn limited_skills_reject_without_consuming_a_turn_and_healing_cannot_rescue() {
    let mut combat = fixture();
    wait_for(&mut combat, ActorId(1));
    combat
        .actor_mut(ActorId(1))
        .expect("hero")
        .skill_uses
        .insert(crate::SkillId::FieldDressing, 2);
    let previous = combat.clone();
    assert_eq!(
        combat.apply(
            ActorId(1),
            CombatAction::Skill {
                skill: crate::SkillId::FieldDressing,
                target: ActorId(1)
            }
        ),
        Err(RuleError::NoUses)
    );
    assert_eq!(combat, previous);
    damage_fixture(&mut combat, ActorId(2), 0);
    combat
        .effect(
            ActorId(4),
            ActorId(2),
            Effect::Heal(8),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("nonstanding later effect skipped");
    assert_eq!(combat.state.actor(ActorId(2)).expect("hero").hp, 0);
}

#[test]
fn snapshot_validation_rejects_malformed_nested_domain_values() {
    let combat = fixture();
    let valid = serde_json::to_value(combat.snapshot()).expect("snapshot JSON");
    for (path, replacement) in [
        ("/actors/0/hp", serde_json::json!(999)),
        ("/actors/0/max_hp", serde_json::json!(999)),
        ("/actors/0/id", serde_json::json!(0)),
        ("/hero_formation/1", serde_json::json!(1)),
        ("/round", serde_json::json!(0)),
        ("/initiative/0/roll", serde_json::json!(9)),
        ("/initiative/0/total", serde_json::json!(0)),
        ("/active_actor", serde_json::json!(999)),
        ("/phase", serde_json::json!("TurnStart")),
    ] {
        let mut malformed = valid.clone();
        *malformed.pointer_mut(path).expect("existing JSON path") = replacement;
        assert!(
            serde_json::from_value::<CombatSnapshot>(malformed).is_err(),
            "rejected {path}"
        );
    }
    let mut with_status = combat;
    attach(
        &mut with_status,
        ActorId(103),
        ActorId(1),
        StatusKind::Bleed,
    );
    let valid_status = serde_json::to_value(with_status.snapshot()).expect("snapshot JSON");
    for (path, replacement) in [
        ("/actors/0/statuses/0/remaining", serde_json::json!(0)),
        ("/actors/0/statuses/0/potency", serde_json::json!(999)),
        ("/actors/0/statuses/0/bearer", serde_json::json!(2)),
        ("/actors/0/statuses/0/source", serde_json::json!(999)),
        (
            "/actors/0/statuses/0/eligible_boundary",
            serde_json::json!(0),
        ),
    ] {
        let mut malformed = valid_status.clone();
        *malformed.pointer_mut(path).expect("existing status path") = replacement;
        assert!(
            serde_json::from_value::<CombatSnapshot>(malformed).is_err(),
            "rejected {path}"
        );
    }
}

#[test]
fn public_snapshot_after_bleed_is_inert_and_reconnect_does_not_repeat_tick() {
    let mut combat = fixture();
    attach(&mut combat, ActorId(103), ActorId(1), StatusKind::Bleed);
    wait_for(&mut combat, ActorId(1));
    let before = combat.clone();
    let wire = serde_json::to_string(&combat.snapshot()).expect("serialize committed decision");
    for _ in 0..4 {
        let reconnected: CombatSnapshot =
            serde_json::from_str(&wire).expect("validated inert projection");
        assert_eq!(reconnected, combat.snapshot());
        assert_eq!(reconnected.active_actor, Some(ActorId(1)));
        assert_eq!(reconnected.phase, CombatPhase::AwaitingAction);
    }
    assert_eq!(
        combat, before,
        "snapshot access/deserialization cannot execute reducer hooks"
    );
    // This proves the pure projection boundary, not an actual process reconnection.
}

#[test]
fn scripted_party_and_ai_complete_identically_without_bevy() {
    let mut left = fixture();
    let mut right = fixture();
    let mut event_ids = std::collections::BTreeSet::new();
    let mut humans_acted = std::collections::BTreeSet::new();
    for _ in 0..400 {
        if left.state.outcome.is_some() {
            break;
        }
        let actor = left.state.active_actor.expect("active");
        let action = left.ai_action().unwrap_or_else(|| {
            humans_acted.insert(actor);
            left.legal_actions(actor)
                .into_iter()
                .filter_map(|action| match action {
                    CombatAction::Skill { skill, target } => {
                        let definition = skill_definition(skill);
                        let damage = definition
                            .effects
                            .iter()
                            .filter_map(|effect| {
                                if let Effect::Damage(value) = effect {
                                    Some(*value)
                                } else {
                                    None
                                }
                            })
                            .sum::<u16>();
                        (damage > 0).then_some((
                            (
                                damage,
                                std::cmp::Reverse(left.state.actor(target).expect("target").hp),
                            ),
                            action,
                        ))
                    }
                    _ => None,
                })
                .max_by_key(|(score, _)| *score)
                .map_or(CombatAction::Wait, |(_, action)| action)
        });
        assert_eq!(left.state.validate_action(actor, &action), Ok(()));
        let events = left.apply(actor, action).expect("legal reducer action");
        assert_eq!(
            events,
            right.apply(actor, action).expect("deterministic replay")
        );
        assert_eq!(left, right);
        left.snapshot().validate().expect("committed invariant");
        for event in events {
            assert!(event_ids.insert(event.id));
        }
    }
    assert_eq!(humans_acted.len(), 4);
    assert_eq!(
        left.state.outcome,
        Some(CombatOutcome::Victory),
        "scripted four-hero party can win the authored encounter"
    );
    let terminal = left.clone();
    assert_eq!(
        left.apply(ActorId(1), CombatAction::Wait),
        Err(RuleError::Finished)
    );
    assert_eq!(left, terminal);
}

#[test]
fn all_heroes_downed_is_immediate_defeat_and_no_enemy_actions_remain() {
    let mut combat = fixture();
    for actor in (1..=4).map(ActorId) {
        damage_fixture(&mut combat, actor, 0);
    }
    assert_eq!(combat.state.outcome, Some(CombatOutcome::Defeat));
    assert_eq!(combat.state.phase, CombatPhase::Finished);
    assert_eq!(combat.state.active_actor, None);
    assert_eq!(combat.ai_action(), None);
    combat.snapshot().validate().expect("terminal projection");
}

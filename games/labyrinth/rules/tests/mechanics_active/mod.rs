use crate::mechanics_support::*;
use labyrinth_rules::{ActorId, CombatAction, LifeState, StatusKind};

#[derive(Clone, Copy)]
enum Expected {
    Hit(u16),
    Bleed(u16),
    Push(u16, u8),
    Pull,
    Cleave,
    Heal(u16),
    Cleanse,
    Rescue,
    Exchange,
}
use Expected::*;

struct Case {
    key: &'static str,
    weapon: Option<&'static str>,
    source: u8,
    target: u8,
    expected: Expected,
    uses: Option<u8>,
}
impl Case {
    fn input(
        &self,
        rank: usize,
    ) -> (
        labyrinth_rules::catalog::ContentCatalog,
        labyrinth_rules::scenario::Scenario,
    ) {
        let personal = if self.weapon.is_none() {
            vec![self.key]
        } else {
            vec![]
        };
        let (catalog, mut input) = scenario(self.weapon, &personal, rank, &[1, 1, 1, 1, 1, 1]);
        if self.key == "assassin_feint" {
            input
                .heroes
                .iter_mut()
                .find(|a| a.id == ActorId(1))
                .expect("source")
                .actor
                .build
                .weapon = Some(id("dagger"));
        }
        (catalog, input)
    }
    fn source_rank(&self) -> usize {
        usize::try_from(self.source.trailing_zeros() + 1).expect("rank")
    }
    fn target_rank(&self) -> u16 {
        u16::try_from(self.target.trailing_zeros() + 1).expect("rank")
    }
    fn target_id(&self) -> u16 {
        match self.expected {
            Heal(_) | Cleanse if matches!(self.key, "field_dressing" | "clean_blade") => 1,
            Heal(_) | Cleanse | Rescue | Exchange => 6,
            _ => 100 + self.target_rank(),
        }
    }
}

fn effects(c: &Case) {
    let (catalog, mut input) = c.input(c.source_rank());
    let target = c.target_id();
    if matches!(c.expected, Heal(_) | Cleanse | Rescue) {
        let recipient = input
            .heroes
            .iter_mut()
            .find(|a| a.id == ActorId(target))
            .expect("ally");
        recipient.starting_hp = Some(if matches!(c.expected, Rescue) { 0 } else { 40 });
        if matches!(c.expected, Cleanse) {
            recipient
                .starting_statuses
                .push(labyrinth_rules::scenario::StartingStatus {
                    kind: StatusKind::Bleed,
                    source: Some(ActorId(1)),
                    remaining: Some(3),
                });
        }
    }
    let mut game = combat(&catalog, &input);
    let before = game.snapshot();
    let command = action(&game, c.key, target);
    let preview = game
        .snapshot()
        .preview_action(ActorId(1), &command)
        .expect("preview");
    let events = commit(&mut game, command);
    let after = game.snapshot();
    let recipient = after.actor(ActorId(target)).expect("target");
    match c.expected {
        Hit(amount) | Bleed(amount) | Push(amount, _) => assert_eq!(recipient.hp, 100 - amount),
        Pull => assert_eq!(recipient.hp, 97),
        Cleave => {
            assert_eq!(after.actor(ActorId(101)).expect("front").hp, 94);
            assert_eq!(after.actor(ActorId(102)).expect("second").hp, 94);
            assert_eq!(after.actor(ActorId(103)).expect("outside pair").hp, 100);
        }
        Heal(amount) => assert_eq!(recipient.hp, 40 + amount),
        Cleanse => {
            assert_eq!(
                recipient.hp,
                before.actor(ActorId(target)).expect("target").hp
            );
            assert!(recipient.statuses.is_empty());
        }
        Rescue => {
            assert_eq!(recipient.hp, 50);
            assert_eq!(recipient.life, LifeState::Alive);
        }
        Exchange => {
            assert_eq!(after.ranks(ActorId(1)), before.ranks(ActorId(target)));
            assert_eq!(after.ranks(ActorId(target)), before.ranks(ActorId(1)));
        }
    }
    match c.expected {
        Bleed(_) => {
            assert_eq!(recipient.statuses.len(), 1);
            let bleed = recipient.statuses.first().expect("Bleed applied");
            assert_eq!(
                (bleed.kind, bleed.potency, bleed.remaining, bleed.source),
                (StatusKind::Bleed, 2, 3, ActorId(1))
            );
        }
        Push(_, distance) => assert_eq!(
            after.rank(ActorId(target)),
            Some(u8::try_from(c.target_rank()).expect("rank") + distance)
        ),
        Pull => assert_eq!(after.rank(ActorId(target)), Some(2)),
        _ => {}
    }
    if !matches!(c.expected, Bleed(_)) {
        assert!(recipient.statuses.is_empty(), "no unauthored status");
    }
    if !matches!(c.expected, Push(_, _) | Pull | Exchange) {
        assert_eq!(after.hero_formation, before.hero_formation);
        assert_eq!(after.enemy_formation, before.enemy_formation);
    }
    // Forecast agreement supplements the independently specified HP/position assertions.
    for projected in &preview.damage {
        assert!(events.iter().any(|event| matches!(event.kind, labyrinth_rules::CombatEventKind::Damage { target, amount, .. } if target == projected.target && amount == projected.hp_loss)));
    }
    let source = after.actor(ActorId(1)).expect("source");
    let index = source.skill_index(&id(c.key)).expect("frozen skill");
    assert_eq!(
        source.remaining_skill_uses(index),
        c.uses.map(|uses| uses - 1)
    );
    assert_eq!(
        source.resolved_build,
        before.actor(ActorId(1)).expect("source").resolved_build
    );
    for other in &after.actors {
        if other.id != ActorId(target)
            && !(matches!(c.expected, Cleave) && other.id == ActorId(102))
        {
            assert_eq!(
                other.hp,
                before.actor(other.id).expect("existing actor").hp,
                "unaffected actor {:?}",
                other.id
            );
        }
    }
}

fn source_ranks(c: &Case) {
    for rank in 1..=6 {
        let (catalog, mut input) = c.input(rank);
        if matches!(c.expected, Rescue) {
            input
                .heroes
                .iter_mut()
                .find(|a| a.id == ActorId(6))
                .expect("ally")
                .starting_hp = Some(0);
        }
        let mut game = combat(&catalog, &input);
        let command = action(&game, c.key, c.target_id());
        if c.source & (1 << (rank - 1)) != 0 {
            assert!(
                game.legal_actions(ActorId(1)).contains(&command),
                "{} source rank {rank}",
                c.key
            );
        } else {
            reject(&mut game, command);
        }
        assert!(
            game.snapshot()
                .actor(ActorId(1))
                .expect("source")
                .skill_index(&id(c.key))
                .is_some(),
            "rank cannot remove Moveset membership"
        );
    }
}

fn target_ranks_and_rejections(c: &Case) {
    let (catalog, mut input) = c.input(c.source_rank());
    if matches!(c.expected, Rescue) {
        input
            .heroes
            .iter_mut()
            .filter(|a| a.id != ActorId(1) && a.id != ActorId(2))
            .for_each(|a| a.starting_hp = Some(0));
    }
    let mut game = combat(&catalog, &input);
    let before = game.snapshot();
    let source = before.actor(ActorId(1)).expect("source");
    let index = source.skill_index(&id(c.key)).expect("skill");
    // Missing IDs and out-of-range indexes must reject transactionally.
    reject(
        &mut game,
        CombatAction::Skill {
            index,
            target: ActorId(999),
        },
    );
    reject(
        &mut game,
        CombatAction::Skill {
            index: u8::MAX,
            target: ActorId(c.target_id()),
        },
    );
    for rank in 1u8..=6 {
        let opponent = 100 + u16::from(rank);
        let command = action(&game, c.key, opponent);
        let offensive = matches!(c.expected, Hit(_) | Bleed(_) | Push(_, _) | Pull | Cleave);
        if offensive && c.target & (1 << (rank - 1)) != 0 {
            assert!(game.legal_actions(ActorId(1)).contains(&command));
        } else {
            reject(&mut game, command);
        }
    }
    let ally = action(&game, c.key, 2);
    match c.expected {
        Heal(_) if c.key != "field_dressing" => {
            assert!(game.legal_actions(ActorId(1)).contains(&ally))
        }
        Cleanse if c.key != "clean_blade" => {
            assert!(game.legal_actions(ActorId(1)).contains(&ally))
        }
        Exchange => assert!(game.legal_actions(ActorId(1)).contains(&ally)),
        _ => reject(&mut game, ally),
    }
    let self_target = action(&game, c.key, 1);
    if matches!(c.expected, Heal(_) | Cleanse) {
        assert!(game.legal_actions(ActorId(1)).contains(&self_target));
    } else {
        reject(&mut game, self_target);
    }
}

fn missing_equipment(c: &Case) {
    let (catalog, mut input) = c.input(c.source_rank());
    input
        .heroes
        .iter_mut()
        .find(|a| a.id == ActorId(1))
        .expect("source")
        .actor
        .build
        .weapon = None;
    let game = combat(&catalog, &input);
    let state = game.snapshot();
    let source = state.actor(ActorId(1)).expect("source");
    assert_eq!(
        source.skill_index(&id(c.key)).is_some(),
        c.weapon.is_none() && c.key != "assassin_feint"
    );
    if c.weapon.is_some() {
        input
            .heroes
            .iter_mut()
            .find(|a| a.id == ActorId(1))
            .expect("source")
            .actor
            .build
            .skills
            .push(labyrinth_rules::build::SkillGrant {
                skill: id(c.key),
                provenance: id("forged"),
            });
        assert!(
            input.validate(&catalog).is_err(),
            "equipment-only content cannot become a personal grant"
        );
    }
}

fn life_targets(c: &Case) {
    let offensive = matches!(c.expected, Hit(_) | Bleed(_) | Push(_, _) | Pull | Cleave);
    let (catalog, mut input) = c.input(c.source_rank());
    if offensive {
        // Allegiance comes from the scenario, not appearance. Enemy-controlled
        // Skills use the identical reducer against a dying hero.
        std::mem::swap(&mut input.heroes, &mut input.enemies);
        input
            .heroes
            .iter_mut()
            .find(|a| a.id == ActorId(c.target_id()))
            .expect("target")
            .starting_hp = Some(0);
        let mut game = combat(&catalog, &input);
        let command = action(&game, c.key, c.target_id());
        let before = game.snapshot();
        commit(&mut game, command);
        let state = game.snapshot();
        let target = state.actor(ActorId(c.target_id())).expect("target");
        assert_eq!(target.life, LifeState::Dying { failures: 1 });
        assert_eq!(target.hp, 0);
        assert!(target.statuses.is_empty(), "no status followup while dying");
        assert_eq!(
            state.ranks(target.id),
            before.ranks(target.id),
            "no displacement while dying"
        );
        corpse_targets(c);
    } else {
        // A different ally may be downed; self-only moves cannot target them.
        input
            .heroes
            .iter_mut()
            .find(|a| a.id == ActorId(6))
            .expect("ally")
            .starting_hp = Some(0);
        let mut game = combat(&catalog, &input);
        let command = action(&game, c.key, 6);
        if matches!(c.expected, Rescue | Exchange) {
            commit(&mut game, command);
        } else {
            reject(&mut game, command);
        }
    }
}

fn corpse_targets(c: &Case) {
    if !matches!(c.expected, Hit(_) | Bleed(_) | Push(_, _) | Pull | Cleave) {
        return;
    }
    let (catalog, mut input) = c.input(c.source_rank());
    input
        .enemies
        .iter_mut()
        .find(|a| a.id == ActorId(c.target_id()))
        .expect("target")
        .starting_hp = Some(1);
    let source = input
        .heroes
        .iter_mut()
        .find(|a| a.id == ActorId(1))
        .expect("source");
    if c.key != "hurled_scrap" {
        source
            .actor
            .build
            .skills
            .push(labyrinth_rules::build::SkillGrant {
                skill: id("hurled_scrap"),
                provenance: id("scenario"),
            });
    }
    let mut game = combat(&catalog, &input);
    let lethal = action(&game, "hurled_scrap", c.target_id());
    commit(&mut game, lethal);
    wait_for(&mut game, ActorId(1));
    let before = game.snapshot();
    assert!(before
        .actor(ActorId(c.target_id()))
        .expect("corpse")
        .is_corpse());
    let command = action(&game, c.key, c.target_id());
    commit(&mut game, command);
    let state = game.snapshot();
    let target = state.actor(ActorId(c.target_id())).expect("corpse");
    let damage = match c.expected {
        Hit(n) | Bleed(n) | Push(n, _) => n,
        Pull => 3,
        Cleave => 6,
        _ => unreachable!(),
    };
    assert_eq!(target.health(), (25 - damage, 25));
    assert_eq!(target.hp, 0);
    assert!(
        target.statuses.is_empty(),
        "corpse cannot gain status followups"
    );
    assert_eq!(
        state.enemy_formation, before.enemy_formation,
        "corpse cannot be displaced"
    );
}

macro_rules! case {
    ($name:ident, $weapon:expr, $source:expr, $target:expr, $effect:expr, $uses:expr) => {
        mod $name {
            use super::*;
            const CASE: Case = Case {
                key: stringify!($name),
                weapon: $weapon,
                source: $source,
                target: $target,
                expected: $effect,
                uses: $uses,
            };
            #[test]
            fn resulting_state() {
                effects(&CASE);
            }
            #[test]
            fn source_rank_boundaries() {
                source_ranks(&CASE);
            }
            #[test]
            fn target_boundaries_and_atomic_rejections() {
                target_ranks_and_rejections(&CASE);
            }
            #[test]
            fn equipment_eligibility() {
                missing_equipment(&CASE);
            }
            #[test]
            fn life_state_and_team_eligibility() {
                life_targets(&CASE);
            }
        }
    };
}

// Authored acceptance values. Never derive expected power/ranks from runtime output.
case!(hurled_scrap, None, 63, 63, Hit(2), None);
case!(spare_bandage, None, 63, 63, Heal(3), Some(2));
case!(crushing_blow, None, 15, 15, Hit(7), None);
case!(front_strike, Some("gatekeeper_spear"), 3, 3, Hit(7), None);
case!(long_reach, Some("gatekeeper_spear"), 15, 15, Hit(5), None);
case!(
    driving_blow,
    Some("gatekeeper_spear"),
    3,
    3,
    Push(3, 2),
    None
);
case!(field_dressing, None, 63, 63, Heal(8), Some(2));
case!(
    bleeding_cut,
    Some("knifehand_daggers"),
    15,
    15,
    Bleed(3),
    None
);
case!(deep_strike, Some("knifehand_daggers"), 15, 3, Hit(6), None);
case!(
    thrown_knife,
    Some("knifehand_daggers"),
    60,
    63,
    Hit(4),
    None
);
case!(clean_blade, None, 63, 63, Cleanse, None);
case!(back_rank_shot, Some("scout_bow"), 60, 48, Hit(7), None);
case!(snap_shot, Some("scout_bow"), 63, 63, Hit(4), None);
case!(hook_shot, Some("scout_bow"), 60, 60, Pull, None);
case!(exchange, None, 63, 63, Exchange, None);
case!(mend, None, 60, 63, Heal(8), Some(3));
case!(staunch, None, 63, 63, Cleanse, None);
case!(staff_strike, Some("medic_staff"), 63, 15, Hit(3), None);
case!(rally, None, 63, 63, Rescue, Some(2));
case!(brutal_strike, None, 3, 3, Hit(5), None);
case!(ragged_cut, None, 63, 15, Bleed(2), None);
case!(hollow_bolt, Some("hollow_bow"), 63, 63, Hit(4), None);
case!(dagger_stab, Some("dagger"), 15, 3, Hit(5), None);
case!(dagger_throw, Some("dagger"), 60, 63, Hit(4), None);
case!(greatsword_cleave, Some("greatsword"), 3, 3, Cleave, None);
case!(greatsword_thrust, Some("greatsword"), 15, 3, Hit(8), None);
case!(axe_chop, Some("two_handed_axe"), 3, 3, Hit(10), None);
case!(spear_thrust, Some("spear"), 15, 15, Hit(6), None);
case!(spear_shove, Some("spear"), 15, 3, Push(2, 1), None);
case!(bow_aimed_shot, Some("bow"), 48, 63, Hit(7), None);
case!(bow_quick_shot, Some("bow"), 63, 63, Hit(4), None);
case!(staff_blow, Some("staff"), 63, 15, Hit(3), None);
case!(staff_push, Some("staff"), 15, 15, Push(2, 1), None);
case!(assassin_feint, None, 63, 15, Hit(4), None);

#[test]
fn every_builtin_skill_has_an_explicit_acceptance_case() {
    // Keep additions visible: neither silently skip new Skills nor generate assertions from them.
    let catalog = labyrinth_rules::catalog::ContentCatalog::builtin().expect("catalog");
    let expected: std::collections::BTreeSet<_> = "hurled_scrap spare_bandage crushing_blow front_strike long_reach driving_blow field_dressing bleeding_cut deep_strike thrown_knife clean_blade back_rank_shot snap_shot hook_shot exchange mend staunch staff_strike rally brutal_strike ragged_cut hollow_bolt dagger_stab dagger_throw greatsword_cleave greatsword_thrust axe_chop spear_thrust spear_shove bow_aimed_shot bow_quick_shot staff_blow staff_push assassin_feint".split_whitespace().collect();
    assert_eq!(
        catalog
            .definition()
            .skills
            .iter()
            .map(|s| s.id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
}

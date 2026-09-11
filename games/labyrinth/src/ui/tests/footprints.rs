//! Layout and activation evidence for whole-footprint occupants, not rendered art.

use super::*;
use labyrinth_rules::{LifeState, Team};

#[test]
fn large_actor_controls_keep_one_identity_and_corpse_health_does_not_reflow() {
    for (width, height) in [(1280, 720), (1920, 1080)] {
        let heroes = [
            HeroClass::Gatekeeper,
            HeroClass::Knifehand,
            HeroClass::Scout,
            HeroClass::FieldMedic,
            HeroClass::LanternWagon,
        ];
        let mut combat = Combat::with_party(
            42,
            heroes
                .into_iter()
                .enumerate()
                .map(|(i, h)| HeroSetup::preset(ActorId(i as u16 + 1), h))
                .collect(),
        )
        .expect("large encounter");
        for _ in 0..24 {
            let actor = combat.snapshot().active_actor.expect("actor");
            if actor == ActorId(3) {
                break;
            }
            combat.apply(actor, CombatAction::Wait).expect("wait");
        }
        let mut view = fixture();
        view.combat = Some(combat.snapshot());
        view.players.retain(|p| p.actor.0 <= 5);
        for p in &mut view.players {
            p.hero = heroes.get(usize::from(p.slot)).copied().expect("class");
        }
        let mut app = App::new();
        app.add_plugins(HeadlessUiPlugin::new(width, height))
            .insert_resource(view)
            .add_plugins(LabyrinthUiPlugin);
        app.finish();
        app.cleanup();
        run_frames(&mut app, 5);
        let tile = find_named(app.world_mut(), "Actor 5 Tile").expect("wagon");
        let small = find_named(app.world_mut(), "Actor 4 Tile").expect("medic");
        let geometry = app
            .world()
            .get::<ComputedNode>(tile)
            .expect("layout")
            .size();
        assert!(
            geometry.x
                > app
                    .world()
                    .get::<ComputedNode>(small)
                    .expect("small layout")
                    .size()
                    .x
                    * 1.8
        );
        assert_eq!(
            app.world_mut()
                .query::<&crate::scene::SceneActorAnchor>()
                .iter(app.world())
                .count(),
            10
        );
        for id in [ActorId(5), ActorId(101)] {
            let control =
                find_named(app.world_mut(), &format!("Actor {}", id.0)).expect("one control");
            assert!(visible_control_rect(
                app.world(),
                control,
                Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32))
            )
            .is_some());
        }
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            let s = view.combat.as_mut().expect("snapshot");
            let a = s
                .actors
                .iter_mut()
                .find(|a| a.id == ActorId(5))
                .expect("wagon");
            a.hp = 0;
            a.life = LifeState::Corpse {
                hp: 6,
                max_hp: 8,
                created_round: s.round,
            };
            s.validate().expect("corpse projection");
        }
        run_frames(&mut app, 3);
        assert_eq!(
            app.world()
                .get::<ComputedNode>(tile)
                .expect("layout")
                .size(),
            geometry
        );
        let text = find_named(app.world_mut(), "Actor 5 Summary").expect("HP label");
        assert_eq!(app.world().get::<Text>(text).expect("text").0, "\nEmber\n6");
        let bar = find_named(app.world_mut(), "Actor 5 HP Bar").expect("corpse bar");
        assert_eq!(
            app.world().get::<Node>(bar).expect("bar").width,
            Val::Percent(75.0)
        );
        apply_action(
            app.world_mut(),
            Action::Choice(Choice::Skill(SkillId::SnapShot)),
        );
        let control = find_named(app.world_mut(), "Actor 5").expect("corpse");
        assert!(click_action(&mut app, control));
        run_frames(&mut app, 2);
        assert_eq!(app.world().resource::<UiState>().target, Some(ActorId(5)));
        let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
        assert!(
            app.world().get::<UiDisabled>(confirm).is_none(),
            "friendly corpse is damageable"
        );
        let s = app
            .world()
            .resource::<LabyrinthView>()
            .combat
            .as_ref()
            .expect("snapshot");
        assert_eq!(s.occupant(Team::Heroes, 6), Some(ActorId(5)));
        let forecast = find_named(app.world_mut(), "Actor 5 HP Forecast").expect("forecast bar");
        assert_eq!(
            app.world().get::<Node>(forecast).expect("bar").width,
            Val::Percent(50.0)
        );
        assert!(app
            .world()
            .get::<bevy::prelude::AccessibleLabel>(control)
            .expect("label")
            .0
            .contains("Corpse durability 6 → 2 / 8"));
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            let snapshot = view.combat.as_mut().expect("snapshot");
            snapshot
                .actors
                .iter_mut()
                .find(|a| a.id == ActorId(5))
                .expect("wagon")
                .life = LifeState::Corpse {
                hp: 1,
                max_hp: 8,
                created_round: snapshot.round,
            };
            snapshot.revision += 1;
        }
        run_frames(&mut app, 3);
        assert_eq!(
            app.world().get::<Node>(forecast).expect("bar").width,
            Val::Percent(12.5)
        );
        assert!(app
            .world()
            .get::<bevy::prelude::AccessibleLabel>(control)
            .expect("label")
            .0
            .contains("Corpse durability 1 → 0 / 8"));
        app.world_mut()
            .resource_mut::<crate::presentation::CombatDisclosure>()
            .actors
            .insert(
                ActorId(5),
                crate::presentation::ActorDisclosure {
                    health: false,
                    ..Default::default()
                },
            );
        run_frames(&mut app, 3);
        assert_eq!(
            app.world()
                .get::<Node>(forecast)
                .expect("hidden forecast")
                .display,
            Display::None
        );
        assert!(!app
            .world()
            .get::<bevy::prelude::AccessibleLabel>(control)
            .expect("label")
            .0
            .contains("Corpse durability 1"));
    }
}

#[test]
fn corpse_help_uses_round_end_and_revokes_hidden_content() {
    use crate::presentation::{ActorDisclosure, CombatDisclosure};
    use bevy_gamekit::ui::{UiTooltipCatalog, UiTooltipOpen, UiTooltipRequest, UiTooltipState};
    let mut app = app(1280, 720, UiScaleMode::Auto);
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("snapshot");
        let enemy = snapshot
            .actors
            .iter_mut()
            .find(|a| a.id == ActorId(101))
            .expect("enemy");
        enemy.hp = 0;
        enemy.life = LifeState::Corpse {
            hp: 1,
            max_hp: 5,
            created_round: snapshot.round,
        };
        enemy.statuses = vec![StatusInstance {
            id: 900,
            kind: StatusKind::Bleed,
            bearer: enemy.id,
            source: ActorId(2),
            potency: 2,
            remaining: 2,
            eligible_boundary: snapshot.boundary_sequence + 1,
        }];
        snapshot.validate().expect("valid fixture");
        snapshot.revision += 1;
    }
    run_frames(&mut app, 3);
    let badge = find_named(app.world_mut(), "Actor 101 Effects").expect("corpse badge");
    let subject = app
        .world()
        .get::<UiTooltipOpen>(badge)
        .expect("help key")
        .0
        .clone();
    let content = app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .get(&subject)
        .expect("help");
    assert!(content
        .facts
        .join(" ")
        .contains("2 damage at round end; up to 2 round-end ticks"));
    assert!(!content.facts.join(" ").contains("at turn start"));
    let definition = app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .get(&content.links.first().expect("definition link").subject)
        .expect("definition");
    assert!(definition
        .body
        .contains("On a corpse, damage and duration use round end"));
    assert!(click_action(&mut app, badge));
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .contains(&subject));
    app.world_mut()
        .resource_mut::<CombatDisclosure>()
        .actors
        .insert(
            ActorId(101),
            ActorDisclosure {
                statuses: false,
                health: false,
                ..Default::default()
            },
        );
    run_frames(&mut app, 3);
    assert!(!app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .contains_key(&subject));
    assert!(!app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .contains(&subject));
    app.world_mut().write_message(UiTooltipRequest::Dismiss);
}

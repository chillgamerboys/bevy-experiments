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
        assert_eq!(app.world().get::<Text>(text).expect("text").0, "H5\n6");
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
    }
}

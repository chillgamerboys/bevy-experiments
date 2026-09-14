//! Layout and activation evidence for whole-footprint occupants, not rendered art.

use super::*;
use labyrinth_rules::{LifeState, Team};

#[test]
fn sparse_formations_keep_six_rank_scale_and_advance_after_removal_normal_1080() {
    fn rect(app: &mut App, name: &str) -> Rect {
        let entity = find_named(app.world_mut(), name).expect(name);
        let node = app.world().get::<ComputedNode>(entity).expect("layout");
        let transform = app
            .world()
            .get::<UiGlobalTransform>(entity)
            .expect("position");
        Rect::from_center_size(
            transform.translation * node.inverse_scale_factor,
            node.size() * node.inverse_scale_factor,
        )
    }
    fn same_size(a: Rect, b: Rect) {
        assert!(
            (a.size() - b.size()).abs().max_element() <= 1.0,
            "{a:?} -> {b:?}"
        );
    }
    fn mount(view: LabyrinthView) -> App {
        let mut app = App::new();
        app.add_plugins(HeadlessUiPlugin::new(1920, 1080))
            .insert_resource(UiScalePreference(UiScaleMode::Auto))
            .insert_resource(view)
            .add_plugins(LabyrinthUiPlugin);
        app.finish();
        app.cleanup();
        run_frames(&mut app, 5);
        app
    }
    let mut combat = Combat::with_party(
        42,
        labyrinth_rules::PROTOTYPE_HERO_ROSTER
            .into_iter()
            .enumerate()
            .map(|(i, hero)| HeroSetup::preset(ActorId(i as u16 + 1), hero))
            .collect(),
    )
    .expect("full six-rank party");
    for _ in 0..24 {
        let active = combat.snapshot().active_actor.expect("active");
        if active == ActorId(1) {
            break;
        }
        combat.apply(active, CombatAction::Wait).expect("wait");
    }
    let mut view = fixture();
    view.combat = Some(combat.snapshot());
    let mut app = mount(view);
    let tracked = [
        "Actor 5 Tile",
        "Actor 101 Tile",
        "Actor 5",
        "Actor 101",
        "Confirm Combat Action",
        "Battle Settings",
    ];
    let before = tracked.map(|name| rect(&mut app, name));
    let removed = [ActorId(2), ActorId(3), ActorId(103), ActorId(104)];
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        for actor in &mut snapshot.actors {
            if removed.contains(&actor.id) {
                actor.hp = 0;
                actor.life = LifeState::Corpse {
                    hp: 1,
                    max_hp: actor.max_hp.div_ceil(4),
                    created_round: snapshot.round,
                };
                actor.statuses.clear();
            }
        }
        snapshot.revision += 1;
        snapshot.validate().expect("corpse fixture");
    }
    run_frames(&mut app, 3);
    for (name, before) in tracked.iter().zip(before) {
        assert_eq!(
            rect(&mut app, name),
            before,
            "death alone retains all ranks"
        );
    }
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        for actor in &mut snapshot.actors {
            if removed.contains(&actor.id) {
                actor.life = LifeState::Removed;
            }
        }
        snapshot.hero_formation.retain(|id| !removed.contains(id));
        snapshot.enemy_formation.retain(|id| !removed.contains(id));
        snapshot.revision += 1;
        snapshot.validate().expect("compacted fixture");
    }
    run_frames(&mut app, 3);
    let after = tracked.map(|name| rect(&mut app, name));
    for (a, b) in before.into_iter().zip(after) {
        same_size(a, b);
    }
    assert!(
        after[0].center().x > before[0].center().x,
        "heroes advance right"
    );
    assert!(
        after[1].center().x < before[1].center().x,
        "enemies advance left"
    );
    assert_eq!(&before[4..], &after[4..], "controls stay in place");
    for team in [Team::Heroes, Team::Enemies] {
        for rank in [5, 6] {
            let empty = find_named(app.world_mut(), &format!("{team:?} Empty Rank {rank}"))
                .expect("back rank");
            assert_eq!(
                app.world().get::<Node>(empty).expect("rank").display,
                Display::Flex
            );
            assert!(app.world().get::<Button>(empty).is_none());
        }
    }
    // Initial sparse rosters have no removed entities to reserve their space.
    let mut sparse = app.world().resource::<LabyrinthView>().clone();
    let snapshot = sparse.combat.as_mut().expect("combat");
    snapshot.actors.retain(|actor| !removed.contains(&actor.id));
    snapshot
        .initiative
        .retain(|entry| !removed.contains(&entry.actor));
    snapshot.validate().expect("initial sparse fixture");
    let mut fresh = mount(sparse);
    for (name, expected) in tracked.iter().zip(after) {
        let actual = rect(&mut fresh, name);
        same_size(actual, expected);
        assert!((actual.center() - expected.center()).abs().max_element() <= 1.0);
    }
    apply_action(fresh.world_mut(), Action::Choice(Choice::Reposition));
    apply_action(fresh.world_mut(), Action::Actor(ActorId(4)));
    run_frames(&mut fresh, 3);
    let markers = [1, 4, 5].map(|id| rect(&mut fresh, &format!("Actor {id} Landing Marker")));
    {
        let mut view = fresh.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        snapshot.hero_formation.swap(0, 1);
        snapshot.revision += 1;
        snapshot.validate().expect("repositioned projection");
    }
    run_frames(&mut fresh, 3);
    for (id, marker) in [1, 4, 5].into_iter().zip(markers) {
        let tile = rect(&mut fresh, &format!("Actor {id} Tile"));
        assert!(
            (marker.center().x - tile.center().x).abs() <= 1.0,
            "preview destination for {id}"
        );
        assert!(
            (marker.width() - tile.width()).abs() <= 1.0,
            "preview footprint for {id}"
        );
    }
}

#[test]
fn large_actor_controls_keep_one_identity_and_corpse_health_does_not_reflow_normal_1080() {
    large_actor_controls_keep_one_identity_and_corpse_health_does_not_reflow(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn large_actor_controls_keep_one_identity_and_corpse_health_does_not_reflow_compatibility() {
    large_actor_controls_keep_one_identity_and_corpse_health_does_not_reflow(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn large_actor_controls_keep_one_identity_and_corpse_health_does_not_reflow(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
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
    view.company.retain(|member| member.actor.0 <= 5);
    for member in &mut view.company {
        member.hero = heroes
            .get(usize::from(member.owner))
            .copied()
            .expect("class");
        member.abilities = resolved_legacy(
            HeroSetup::preset(member.actor, member.hero)
                .abilities
                .as_slice(),
        );
    }
    for player in &mut view.players {
        player.actors.retain(|actor| actor.0 <= 5);
    }
    let mut app = App::new();
    app.add_plugins(HeadlessUiPlugin::new(width, height))
        .insert_resource(UiScalePreference(scale))
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
        let control = find_named(app.world_mut(), &format!("Actor {}", id.0)).expect("one control");
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
    assert_eq!(
        app.world().get::<Text>(text).expect("text").0,
        "Lantern Wagon\n6"
    );
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

#[test]
fn corpse_help_uses_round_end_and_revokes_hidden_content_normal_1080() {
    corpse_help_uses_round_end_and_revokes_hidden_content(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn corpse_help_uses_round_end_and_revokes_hidden_content_compatibility() {
    corpse_help_uses_round_end_and_revokes_hidden_content(1280, 720, UiScaleMode::Auto);
}

fn corpse_help_uses_round_end_and_revokes_hidden_content(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    use crate::presentation::{ActorDisclosure, CombatDisclosure};
    use bevy_gamekit::ui::{UiTooltipCatalog, UiTooltipOpen, UiTooltipRequest, UiTooltipState};
    let mut app = app(width, height, scale);
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

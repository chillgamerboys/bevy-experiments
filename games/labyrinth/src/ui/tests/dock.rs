//! Dock interaction and viewer-projection evidence, not rendered-frame evidence.

use super::*;
use crate::presentation::{ActorDisclosure, CombatDisclosure};
use bevy_game_ui::UiContextHelp;
use labyrinth_rules::{skill_definition, Team};

fn text_named(app: &mut App, name: &str) -> String {
    let entity = find_named(app.world_mut(), name).expect("named text");
    app.world().get::<Text>(entity).expect("text").0.clone()
}

fn no_combat_intent(app: &mut App) {
    assert!(!app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .any(|intent| matches!(intent, LabyrinthIntent::Combat { .. })));
}

#[test]
fn rank_numbers_stay_plain_when_showing_ability_range_and_selection() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    let snapshot = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .clone()
        .expect("combat");
    let source = snapshot
        .actor(snapshot.active_actor.expect("active"))
        .expect("actor");
    let skill = *source.skills().first().expect("ability");
    apply_action(app.world_mut(), Action::Choice(Choice::Skill(skill)));
    apply_action(app.world_mut(), Action::Actor(ActorId(101)));
    run_frames(&mut app, 3);
    for actor in &snapshot.actors {
        assert_eq!(
            text_named(&mut app, &format!("Actor {} Formation Cue", actor.id.0)),
            snapshot.rank(actor.id).expect("rank").to_string()
        );
    }
}

#[test]
fn timeline_pointer_and_keyboard_inspection_never_replace_the_selected_target() {
    for keyboard in [false, true] {
        let mut app = app(1280, 720, UiScaleMode::Auto);
        let before = app.world().resource::<LabyrinthView>().combat.clone();
        apply_action(app.world_mut(), Action::Choice(Choice::Wait));
        apply_action(app.world_mut(), Action::Actor(ActorId(103)));
        run_frames(&mut app, 3);
        let portrait =
            find_named(app.world_mut(), "Initiative Actor 2").expect("initiative portrait");
        let prior_focus = app.world().resource::<InputFocus>().get();
        if keyboard {
            assert!(focus_action(app.world_mut(), portrait));
            tap_key(&mut app, KeyCode::Enter);
        } else {
            pointer_control(&mut app, portrait, Vec2::new(1280.0, 720.0));
        }
        run_frames(&mut app, 3);
        let ui = app.world().resource::<UiState>();
        assert_eq!(ui.target, Some(ActorId(103)));
        assert_eq!(ui.selected, Some(Choice::Wait));
        assert_eq!(ui.inspected, Some(ActorId(2)));
        assert!(ui.show_inspector);
        no_combat_intent(&mut app);
        tap_key(&mut app, KeyCode::Escape);
        run_frames(&mut app, 3);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            if keyboard {
                Some(portrait)
            } else {
                prior_focus
            }
        );
        assert_eq!(app.world().resource::<UiState>().target, Some(ActorId(103)));
        assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
    }
}

#[test]
fn timeline_absent_actor_keeps_parentage_and_identity_until_battle_teardown() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let portrait = find_named(app.world_mut(), "Initiative Actor 106").expect("portrait");
    let parent = app
        .world()
        .get::<ChildOf>(portrait)
        .expect("parent")
        .parent();
    let original = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .as_ref()
        .expect("combat")
        .initiative
        .clone();
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .combat
        .as_mut()
        .expect("combat")
        .initiative
        .retain(|entry| entry.actor != ActorId(106));
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().get::<ChildOf>(portrait).map(ChildOf::parent),
        Some(parent)
    );
    assert_eq!(
        app.world().get::<Node>(portrait).expect("node").display,
        Display::None
    );
    assert!(!activation_eligible(app.world_mut(), portrait));
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .combat
        .as_mut()
        .expect("combat")
        .initiative = original;
    run_frames(&mut app, 3);
    assert_eq!(
        find_named(app.world_mut(), "Initiative Actor 106"),
        Some(portrait)
    );
    assert_eq!(
        app.world().get::<Node>(portrait).expect("node").display,
        Display::Flex
    );
    assert!(activation_eligible(app.world_mut(), portrait));
    app.world_mut().resource_mut::<LabyrinthView>().mode = ViewMode::Menu;
    run_frames(&mut app, 3);
    assert!(app.world().get_entity(portrait).is_err());
    assert!(app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .all(|name| !name.as_str().starts_with("Initiative Actor ")));
}

#[test]
fn off_turn_owned_ability_has_a_forecast_but_cannot_commit() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let snapshot = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .clone()
        .expect("combat");
    let (slot, source, index, skill, target) = app
        .world()
        .resource::<LabyrinthView>()
        .players
        .iter()
        .filter(|player| Some(player.actor) != snapshot.active_actor)
        .find_map(|player| {
            let actor = snapshot.actor(player.actor)?;
            actor
                .skills()
                .iter()
                .enumerate()
                .find_map(|(index, skill)| {
                    snapshot
                        .actors
                        .iter()
                        .filter(|target| target.team() == Team::Enemies)
                        .find_map(|target| {
                            let action = CombatAction::Skill {
                                skill: *skill,
                                target: target.id,
                            };
                            snapshot
                                .preview_action(actor.id, &action)
                                .ok()
                                .filter(|preview| !preview.damage.is_empty())
                                .map(|_| (player.slot, actor.id, index, *skill, target.id))
                        })
                })
        })
        .expect("off-turn damage preview");
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.local = false;
        view.player = Some(slot);
    }
    run_frames(&mut app, 3);
    let skill_control = find_named(app.world_mut(), &format!("Skill {index}")).expect("skill");
    assert!(focus_action(app.world_mut(), skill_control));
    tap_key(&mut app, KeyCode::Enter);
    let target_control =
        find_named(app.world_mut(), &format!("Actor {}", target.0)).expect("target");
    pointer_control(&mut app, target_control, Vec2::new(1920.0, 1080.0));
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().resource::<UiState>().selected,
        Some(Choice::Skill(skill))
    );
    let preview = snapshot
        .preview_action(source, &CombatAction::Skill { skill, target })
        .expect("preview");
    let expected_hp = preview.actor(target).expect("target outcome").after.hp;
    let facts = crate::presentation::ForecastDisplay::build(
        &snapshot,
        app.world()
            .resource::<crate::presentation::CombatDisclosure>(),
        source,
        &CombatAction::Skill { skill, target },
    )
    .expect("disclosed preview");
    assert_eq!(
        facts
            .actors
            .iter()
            .find(|change| change.actor == target)
            .and_then(|change| change.health.as_known())
            .expect("health")
            .current,
        expected_hp
    );
    let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
    assert!(app
        .world()
        .get::<bevy_game_ui::UiContextHelp>(confirm)
        .expect("disabled explanation")
        .body
        .contains("Waiting for your turn"));
    assert!(app.world().get::<UiDisabled>(confirm).is_some());
    assert!(!focus_action(app.world_mut(), confirm));
    pointer_control(&mut app, confirm, Vec2::new(1920.0, 1080.0));
    no_combat_intent(&mut app);
    assert_eq!(
        app.world().resource::<LabyrinthView>().combat.as_ref(),
        Some(&snapshot)
    );
}

#[test]
fn every_zero_to_eight_loadout_is_keyboard_reachable_at_both_scales() {
    let shortcuts = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ];
    for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
        let mut app = app(1280, 720, scale);
        let owner = app
            .world()
            .resource::<LabyrinthView>()
            .players
            .get(5)
            .expect("sixth seat")
            .actor;
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            view.local = false;
            view.player = Some(5);
        }
        for count in 0..=MAX_EQUIPPED_ABILITIES {
            {
                let mut view = app.world_mut().resource_mut::<LabyrinthView>();
                let actor = view
                    .combat
                    .as_mut()
                    .expect("combat")
                    .actors
                    .iter_mut()
                    .find(|actor| actor.id == owner)
                    .expect("owned hero");
                actor.abilities =
                    AbilityLoadout::new(SkillId::ALL.into_iter().take(count)).expect("loadout");
                actor.skill_uses.clear();
            }
            run_frames(&mut app, 4);
            assert!(find_named(app.world_mut(), &format!("Skill {count}")).is_none());
            let skills = (0..count)
                .map(|index| {
                    find_named(app.world_mut(), &format!("Skill {index}")).expect("equipped slot")
                })
                .collect::<Vec<_>>();
            for (index, &entity) in skills.iter().enumerate() {
                let skill = *SkillId::ALL.get(index).expect("equipped catalog entry");
                if index == 0 {
                    assert!(focus_action(app.world_mut(), entity));
                } else {
                    tap_key(&mut app, KeyCode::Tab);
                }
                run_frames(&mut app, 4);
                assert_eq!(
                    app.world().resource::<InputFocus>().get(),
                    Some(entity),
                    "{count} abilities, slot {index}, {scale:?}"
                );
                let visible = visible_control_rect(
                    app.world(),
                    entity,
                    Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
                )
                .expect("focused skill is visible");
                assert!(
                    visible.width() >= 43.5 && visible.height() >= 43.5,
                    "{count} abilities, slot {index}, {scale:?}: {visible:?}"
                );
                tap_key(&mut app, KeyCode::Enter);
                assert_eq!(
                    app.world().resource::<UiState>().selected,
                    Some(Choice::Skill(skill))
                );
                assert!(app
                    .world()
                    .get::<AccessibleLabel>(entity)
                    .expect("accessible ability")
                    .0
                    .contains(skill_definition(skill).name));
            }
            for (index, key) in shortcuts.iter().copied().enumerate().take(count) {
                let skill = *SkillId::ALL.get(index).expect("equipped catalog entry");
                tap_key(&mut app, key);
                assert_eq!(
                    app.world().resource::<UiState>().selected,
                    Some(Choice::Skill(skill))
                );
            }
            let wait =
                find_named(app.world_mut(), "Wait").expect("universal remains for empty loadout");
            assert!(focus_action(app.world_mut(), wait));
            tap_key(&mut app, KeyCode::Enter);
            if count == 0 {
                tap_key(&mut app, KeyCode::Digit1);
                assert_eq!(
                    app.world().resource::<UiState>().selected,
                    Some(Choice::Wait)
                );
            }
            no_combat_intent(&mut app);
        }
    }
}

fn presented_strings(app: &mut App) -> Vec<(String, String)> {
    let mut strings = Vec::new();
    for (name, text, accessible, help) in app
        .world_mut()
        .query::<(
            &Name,
            Option<&Text>,
            Option<&AccessibleLabel>,
            Option<&UiContextHelp>,
        )>()
        .iter(app.world())
    {
        for value in [
            text.map(|text| text.0.clone()),
            accessible.map(|label| label.0.clone()),
            help.map(|help| format!("{}\n{}", help.title, help.body)),
        ]
        .into_iter()
        .flatten()
        {
            strings.push((name.as_str().to_owned(), value));
        }
    }
    strings.sort();
    strings
}

#[test]
fn concealed_state_changes_do_not_leak_through_text_accessibility_or_context_help() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let source = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .as_ref()
        .expect("combat")
        .active_actor
        .expect("hero");
    let target = ActorId(103);
    let mut disclosure = CombatDisclosure::default();
    disclosure.actors.insert(
        target,
        ActorDisclosure {
            health: false,
            statuses: false,
            details: false,
        },
    );
    // Partial concealment also hides modifier-derived speed and remaining uses.
    disclosure.actors.insert(
        source,
        ActorDisclosure {
            health: true,
            statuses: false,
            details: true,
        },
    );
    app.insert_resource(disclosure);
    apply_action(app.world_mut(), Action::SkillSlot(0));
    apply_action(app.world_mut(), Action::Actor(target));
    apply_action(app.world_mut(), Action::ToggleInspector);
    app.world_mut().resource_mut::<LabyrinthView>().log =
        vec!["CONCEALED-OUTCOME-ALPHA".to_owned()];
    apply_action(app.world_mut(), Action::InspectActor(source));
    run_frames(&mut app, 3);
    let partial_before = text_named(&mut app, "Inspector Text");
    assert!(partial_before.contains("Speed unknown"));
    assert!(partial_before.contains("Status effects unknown"));
    assert!(!partial_before.contains("HP unknown"));
    apply_action(app.world_mut(), Action::InspectActor(target));
    run_frames(&mut app, 4);
    let before = presented_strings(&mut app);
    let combined = before
        .iter()
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(combined.contains("HP unknown"));
    assert!(combined.contains("Status effects unknown"));
    assert!(combined.contains("Outcome uncertain"));
    assert!(!combined.contains("CONCEALED-OUTCOME-ALPHA"));
    assert!(!combined.contains("uses remaining"));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.log = vec!["CONCEALED-OUTCOME-BETA".to_owned()];
        let combat = view.combat.as_mut().expect("combat");
        let boundary = combat.boundary_sequence;
        for actor in &mut combat.actors {
            if actor.id == target || actor.id == source {
                actor.statuses.push(StatusInstance {
                    id: 900 + u64::from(actor.id.0),
                    kind: StatusKind::Haste,
                    bearer: actor.id,
                    source,
                    potency: 3,
                    remaining: 1,
                    eligible_boundary: boundary + 1,
                });
                for uses in actor.skill_uses.values_mut() {
                    *uses = 1;
                }
                if actor.id == target {
                    actor.hp = 1;
                    actor.base_speed = 37;
                }
            }
        }
        combat.revision += 1;
    }
    run_frames(&mut app, 4);
    assert_eq!(presented_strings(&mut app), before, "Concealed facts must not change any generated text, accessible label, or help content, even on hidden surfaces.");
    apply_action(app.world_mut(), Action::InspectActor(source));
    run_frames(&mut app, 3);
    assert_eq!(text_named(&mut app, "Inspector Text"), partial_before);
    let forecast = find_named(app.world_mut(), "Actor 103 HP Forecast").expect("forecast bar");
    assert_eq!(
        app.world().get::<Node>(forecast).expect("node").display,
        Display::None
    );
    no_combat_intent(&mut app);
}

#[test]
fn replacement_palette_changes_paint_without_changing_layout_selection_or_gameplay() {
    let mut app = app(1280, 720, UiScaleMode::Percent200);
    apply_action(app.world_mut(), Action::Choice(Choice::Wait));
    run_frames(&mut app, 3);
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0));
    let nodes = app
        .world_mut()
        .query_filtered::<Entity, With<Node>>()
        .iter(app.world())
        .map(|entity| (entity, visible_control_rect(app.world(), entity, viewport)))
        .collect::<Vec<_>>();
    let skin = LabyrinthAppearance {
        dock: Color::srgb(0.1, 0.12, 0.15),
        ink: Color::srgb(0.7, 0.9, 1.0),
        accent: Color::srgb(0.35, 0.8, 1.0),
        pressed: Color::srgb(0.12, 0.2, 0.28),
        ..default()
    };
    app.insert_resource(skin.clone());
    run_frames(&mut app, 4);
    for (entity, rect) in nodes {
        assert_eq!(
            visible_control_rect(app.world(), entity, viewport),
            rect,
            "palette changed layout for {:?}",
            app.world().get::<Name>(entity)
        );
    }
    let dock = find_named(app.world_mut(), "Combat Command Dock").expect("dock");
    assert_eq!(
        app.world().get::<BackgroundColor>(dock),
        Some(&BackgroundColor(skin.dock))
    );
    let wait = find_named(app.world_mut(), "Wait").expect("wait");
    assert_eq!(
        app.world().get::<UiSkinOverrides>(wait),
        Some(&skin.control(true))
    );
    assert_eq!(
        app.world().resource::<UiState>().selected,
        Some(Choice::Wait)
    );
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
    no_combat_intent(&mut app);
    let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
    assert!(focus_action(app.world_mut(), confirm));
    tap_key(&mut app, KeyCode::Space);
    let intents = app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(matches!(
        intents.as_slice(),
        [LabyrinthIntent::Combat {
            action: CombatAction::Wait,
            ..
        }]
    ));
}

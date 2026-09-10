//! Live decision/loadout replacement through native UI; not GPU rendering evidence.

use super::*;
use bevy_game_ui::{UiContextHelpState, UiSkinOverrides};
use labyrinth_rules::skill_definition;

#[derive(Resource, Default)]
struct CombatCommands(Vec<(ActorId, CombatAction)>);

fn capture_commands(
    mut intents: MessageReader<LabyrinthIntent>,
    mut captured: ResMut<CombatCommands>,
) {
    for intent in intents.read() {
        if let LabyrinthIntent::Combat { actor, action, .. } = intent {
            captured.0.push((*actor, *action));
        }
    }
}

fn advance_to(combat: &mut Combat, actor: ActorId) {
    for _ in 0..labyrinth_rules::MAX_ACTORS * 2 {
        let active = combat.snapshot().active_actor.expect("active fixture");
        if active == actor {
            return;
        }
        combat.apply(active, CombatAction::Wait).expect("real wait");
    }
    assert_eq!(
        combat.snapshot().active_actor,
        Some(actor),
        "fixture did not reach the requested actor"
    );
}

fn text(app: &App, entity: Entity) -> &str {
    &app.world().get::<Text>(entity).expect("persistent text").0
}

fn activate(app: &mut App, entity: Entity, keyboard: bool) {
    if keyboard {
        assert!(focus_action(app.world_mut(), entity));
        tap_key(app, KeyCode::Enter);
    } else {
        pointer_control(app, entity, Vec2::new(1920.0, 1080.0));
    }
    run_frames(app, 4);
}

#[test]
fn medic_commands_refresh_text_help_and_selection_after_scout_commits() {
    for keyboard in [false, true] {
        for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
            let mut combat = Combat::new(42, DEFAULT_HERO_ROSTER).expect("combat");
            advance_to(&mut combat, ActorId(4));
            let mut app = app(1920, 1080, scale);
            app.init_resource::<CombatCommands>()
                .add_systems(PostUpdate, capture_commands);
            app.world_mut().resource_mut::<LabyrinthView>().combat = Some(combat.snapshot());
            run_frames(&mut app, 5);
            let old_skill = find_named(app.world_mut(), "Skill 0").expect("scout skill");
            activate(&mut app, old_skill, keyboard);
            assert_eq!(
                app.world().resource::<UiState>().selected,
                Some(Choice::Skill(SkillId::BackRankShot))
            );
            let target = find_named(app.world_mut(), "Actor 105").expect("rear enemy");
            activate(&mut app, target, keyboard);
            let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
            assert!(
                activation_eligible(app.world_mut(), confirm),
                "confirm eligibility: keyboard={keyboard}, scale={scale:?}, selected={:?}, target={:?}, tooltip={:?}",
                app.world().resource::<UiState>().selected,
                app.world().resource::<UiState>().target,
                app.world().resource::<bevy_game_ui::UiTooltipState>().subjects()
            );
            activate(&mut app, confirm, keyboard);
            let diagnostics = app
                .world_mut()
                .query::<(
                    Entity,
                    &Name,
                    &ComputedNode,
                    &UiGlobalTransform,
                    Option<&Interaction>,
                )>()
                .iter(app.world())
                .filter(|(entity, name, _, _, interaction)| {
                    *entity == confirm
                        || name.as_str().starts_with("Tooltip Card")
                        || interaction.is_some_and(|interaction| *interaction != Interaction::None)
                })
                .map(|(_, name, node, transform, interaction)| {
                    (
                        name.as_str().to_owned(),
                        node.size(),
                        transform.translation,
                        interaction.copied(),
                    )
                })
                .collect::<Vec<_>>();
            let commands = std::mem::take(&mut app.world_mut().resource_mut::<CombatCommands>().0);
            assert_eq!(
                commands,
                vec![(
                    ActorId(4),
                    CombatAction::Skill {
                        skill: SkillId::BackRankShot,
                        target: ActorId(105),
                    },
                )],
                "keyboard={keyboard}, scale={scale:?}, selected={:?}, target={:?}, confirm={:?}, tooltip={:?}, selection={:?}, geometry={diagnostics:?}",
                app.world().resource::<UiState>().selected,
                app.world().resource::<UiState>().target,
                app.world().get::<Interaction>(confirm),
                app.world().resource::<bevy_game_ui::UiTooltipState>().subjects(),
                battle::selected_action(app.world().resource::<LabyrinthView>(), app.world().resource::<UiState>())
            );
            for (actor, action) in commands {
                combat.apply(actor, action).expect("authoritative action");
            }
            advance_to(&mut combat, ActorId(6));
            let next = combat.snapshot();
            next.validate().expect("valid decision transition");
            app.world_mut().resource_mut::<LabyrinthView>().combat = Some(next.clone());
            run_frames(&mut app, 5);
            assert!(app.world().get_entity(old_skill).is_err());
            assert_eq!(app.world().resource::<UiState>().selected, None);

            for (name, choice, heading) in [
                ("Reposition", Choice::Reposition, "Reposition"),
                ("Skill 0", Choice::Skill(SkillId::Mend), "Mend"),
                ("Skill 1", Choice::Skill(SkillId::Staunch), "Staunch"),
            ] {
                let button = find_named(app.world_mut(), name).expect("medic control");
                activate(&mut app, button, keyboard);
                assert_eq!(app.world().resource::<UiState>().selected, Some(choice));
                assert_eq!(
                    app.world().get::<UiSkinOverrides>(button),
                    Some(&app.world().resource::<LabyrinthAppearance>().control(true)),
                    "{name}: selected control must update its skin"
                );
                assert_eq!(
                    app.world().get::<BorderColor>(button),
                    Some(&BorderColor::all(
                        app.world().resource::<LabyrinthAppearance>().accent
                    )),
                    "{name}: skin must reach the rendered control component"
                );
                let state = app.world().resource::<UiContextHelpState>();
                assert_eq!(state.entity, Some(button));
                let content = state.content.as_ref().expect("current medic help");
                assert!(content.title.contains(heading) || content.body.contains(heading));
                run_frames(&mut app, 8);
                let title = find_named(app.world_mut(), "Tooltip Title");
                assert!(title.is_some(), "current tooltip: {name}, keyboard={keyboard}, scale={scale:?}, source={:?}, state={:?}, help={:?}",
                    app.world().get::<bevy_game_ui::UiTooltipSource>(button),
                    app.world().resource::<bevy_game_ui::UiTooltipState>().subjects(),
                    app.world().resource::<UiContextHelpState>());
                let title = title.expect("current tooltip");
                assert!(!text(&app, title).contains(skill_definition(SkillId::BackRankShot).name));
                assert_eq!(
                    app.world().resource::<LabyrinthView>().combat.as_ref(),
                    Some(&next)
                );
                assert!(app.world().resource::<CombatCommands>().0.is_empty());
            }
        }
    }
}

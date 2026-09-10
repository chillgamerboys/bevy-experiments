//! Production UI/input/scene-geometry regressions; not a substitute for pixel review.

use super::*;
use crate::scene::{SceneActorAnchor, SceneAppearance};
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy_game_ui::UiContextHelp;
use labyrinth_rules::{skill_definition, CombatSnapshot};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
struct ActorGeometry {
    anchor: Rect,
    transform: Transform,
    sprite_size: Option<Vec2>,
}

#[derive(Resource, Default)]
struct CapturedCombatIntents(usize);

fn capture_combat_intents(
    mut intents: MessageReader<LabyrinthIntent>,
    mut captured: ResMut<CapturedCombatIntents>,
) {
    captured.0 += intents
        .read()
        .filter(|intent| matches!(intent, LabyrinthIntent::Combat { .. }))
        .count();
}

/// Decode the real actor sheet synchronously so a background asset load cannot
/// switch primitive/art render branches midway through a geometry assertion.
fn scene_app(width: u32, height: u32, scale: UiScaleMode) -> App {
    let mut app = app(width, height, scale);
    app.init_resource::<CapturedCombatIntents>().add_systems(
        Update,
        capture_combat_intents.after(LabyrinthUiSystems::Input),
    );
    let image = Image::from_buffer(
        include_bytes!("../../scene/assets/actors.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::all(),
    )
    .expect("bundled actor art decodes");
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.world_mut()
        .resource_mut::<SceneAppearance>()
        .actor_sheet = Some(handle);
    // Non-full, two-digit HP exercises both healing and damage forecast labels.
    for actor in &mut app
        .world_mut()
        .resource_mut::<LabyrinthView>()
        .combat
        .as_mut()
        .expect("combat")
        .actors
    {
        actor.hp = actor.hp.min(10);
    }
    run_frames(&mut app, 5);
    app
}

fn geometry(app: &mut App) -> BTreeMap<ActorId, ActorGeometry> {
    let anchors = app
        .world_mut()
        .query::<(Entity, &SceneActorAnchor)>()
        .iter(app.world())
        .map(|(entity, anchor)| (anchor.actor, entity))
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    for (actor, entity) in anchors {
        let computed = app.world().get::<ComputedNode>(entity).expect("layout");
        let ui_transform = app
            .world()
            .get::<UiGlobalTransform>(entity)
            .expect("UI transform");
        let half = computed.size() * 0.5;
        let anchor = Rect::from_corners(
            ui_transform.transform_point2(-half) * computed.inverse_scale_factor,
            ui_transform.transform_point2(half) * computed.inverse_scale_factor,
        );
        assert!(anchor.width() >= 44.0 && anchor.height() >= 44.0);
        let body = find_named(app.world_mut(), &format!("Scene actor {}", actor.0))
            .expect("world actor sprite");
        let sprite = app.world().get::<Sprite>(body).expect("sprite");
        assert!(
            sprite.rect.is_some(),
            "exercise the actual actor-sheet branch"
        );
        result.insert(
            actor,
            ActorGeometry {
                anchor,
                transform: *app.world().get::<Transform>(body).expect("world transform"),
                sprite_size: sprite.custom_size,
            },
        );
    }
    assert_eq!(
        result.len(),
        12,
        "all six heroes and six enemies stay present"
    );
    result
}

fn unchanged(
    app: &mut App,
    expected: &BTreeMap<ActorId, ActorGeometry>,
    before: &CombatSnapshot,
    description: &str,
) {
    run_frames(app, 3);
    assert_eq!(&geometry(app), expected, "{description}");
    preview_text_fits(app, description);
    assert_eq!(
        app.world().resource::<LabyrinthView>().combat.as_ref(),
        Some(before),
        "inspection never changes combat: {description}"
    );
    // A per-frame reader retains this evidence across settling frames; draining
    // Bevy's short-lived message buffer here could miss an already-aged command.
    assert_eq!(
        app.world().resource::<CapturedCombatIntents>().0,
        0,
        "inspection never commits: {description}"
    );
}

fn preview_text_fits(app: &mut App, description: &str) {
    let window = app
        .world_mut()
        .query::<&Window>()
        .single(app.world())
        .expect("one window");
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(window.width(), window.height()));
    for name in ["Command Hero Identity"] {
        let entity = find_named(app.world_mut(), name).expect("preview text");
        let text = app.world().get::<Text>(entity).expect("text");
        if text.0.is_empty() {
            continue;
        }
        let layout = app
            .world()
            .get::<bevy::text::TextLayoutInfo>(entity)
            .expect("real shaped preview text");
        assert!(!layout.glyphs.is_empty(), "{name} must use measured text");
        let computed = app
            .world()
            .get::<ComputedNode>(entity)
            .expect("text bounds");
        let visible = visible_control_rect(app.world(), entity, viewport).expect("visible text");
        let measured = layout.size * computed.inverse_scale_factor;
        assert!(
            measured.x <= visible.width() + 1.0 && measured.y <= visible.height() + 1.0,
            "essential {name} clipped: {description}: {:?}, measured {measured:?}, visible {visible:?}",
            text.0
        );
    }
}

fn keyboard_control(app: &mut App, name: &str) {
    let entity = find_named(app.world_mut(), name).expect("named action");
    assert!(focus_action(app.world_mut(), entity), "{name} is focusable");
    tap_key(app, KeyCode::Enter);
    run_frames(app, 3);
}

fn hover_at(app: &mut App, point: Vec2) {
    let (window, mut value) = app
        .world_mut()
        .query::<(Entity, &mut Window)>()
        .single_mut(app.world_mut())
        .expect("one window");
    value.set_cursor_position(Some(point));
    // Winit emits the aggregate stream used by picking in addition to the
    // Window cursor state used by legacy native UI Interaction.
    app.world_mut()
        .write_message(bevy::window::WindowEvent::CursorMoved(
            bevy::window::CursorMoved {
                window,
                position: point,
                delta: None,
            },
        ));
    run_frames(app, 8);
}

fn native_pointer_click(app: &mut App, point: Vec2) {
    use bevy::input::{mouse::MouseButtonInput, ButtonState};
    hover_at(app, point);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .expect("window");
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = MouseButtonInput {
            window,
            button: MouseButton::Left,
            state,
        };
        app.world_mut().write_message(event);
        app.world_mut()
            .write_message(bevy::window::WindowEvent::MouseButtonInput(event));
        app.update();
    }
}

fn hover_control(app: &mut App, entity: Entity, viewport: Rect) {
    let rect = visible_control_rect(app.world(), entity, viewport).expect("hoverable control");
    hover_at(app, rect.center());
    assert_eq!(
        app.world().get::<Interaction>(entity),
        Some(&Interaction::Hovered)
    );
}

#[test]
fn overlay_selection_forecasts_and_drawers_never_move_world_characters() {
    for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
        for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
            let mut app = scene_app(width, height, scale);
            let snapshot = app
                .world()
                .resource::<LabyrinthView>()
                .combat
                .clone()
                .expect("combat");
            let source = snapshot.active_actor.expect("acting hero");
            let skills = snapshot.actor(source).expect("hero").skills().to_vec();
            let expected = geometry(&mut app);
            for (index, skill) in skills.into_iter().enumerate() {
                keyboard_control(&mut app, &format!("Skill {index}"));
                assert_eq!(
                    app.world().resource::<UiState>().selected,
                    Some(Choice::Skill(skill))
                );
                unchanged(
                    &mut app,
                    &expected,
                    &snapshot,
                    &format!("{width}x{height} {scale:?}: selecting {skill:?}"),
                );
                // A valid target may not exist when a skill is unavailable from
                // the current rank. Invalid inspection must still stay stable.
                for valid in [true, false] {
                    let target = snapshot.actors.iter().find(|actor| {
                        snapshot
                            .validate_action_target(
                                source,
                                &CombatAction::Skill {
                                    skill,
                                    target: actor.id,
                                },
                            )
                            .is_ok()
                            == valid
                    });
                    if let Some(target) = target {
                        keyboard_control(&mut app, &format!("Actor {}", target.id.0));
                        assert_eq!(app.world().resource::<UiState>().target, Some(target.id));
                        unchanged(
                            &mut app,
                            &expected,
                            &snapshot,
                            &format!("{width}x{height} {scale:?}: {skill:?}, valid={valid}"),
                        );
                    }
                }
                let selection = app.world().resource::<UiState>().selected;
                let target = app.world().resource::<UiState>().target;
                for toggle in ["Inspector Toggle", "Battle Log Toggle", "Timeline Toggle"] {
                    let button = find_named(app.world_mut(), toggle).expect("drawer toggle");
                    pointer_control(&mut app, button, Vec2::new(width as f32, height as f32));
                    let ui = app.world().resource::<UiState>();
                    assert!(ui.show_inspector || ui.show_log || ui.show_timeline);
                    unchanged(&mut app, &expected, &snapshot, toggle);
                    tap_key(&mut app, KeyCode::Escape);
                    unchanged(
                        &mut app,
                        &expected,
                        &snapshot,
                        "closing selected-state drawer",
                    );
                    let ui = app.world().resource::<UiState>();
                    assert_eq!(ui.selected, selection);
                    assert_eq!(ui.target, target);
                    assert!(!ui.show_inspector && !ui.show_log && !ui.show_timeline);
                }
                keyboard_control(&mut app, "Cancel Combat Selection");
                assert!(app.world().resource::<UiState>().selected.is_none());
                unchanged(&mut app, &expected, &snapshot, "cancel ability and target");
            }
            for (name, choice) in [("Wait", Choice::Wait), ("Defend", Choice::Defend)] {
                keyboard_control(&mut app, name);
                assert_eq!(app.world().resource::<UiState>().selected, Some(choice));
                unchanged(&mut app, &expected, &snapshot, name);
                keyboard_control(&mut app, "Cancel Combat Selection");
                unchanged(&mut app, &expected, &snapshot, "cancel utility");
            }
        }
    }
}

#[test]
fn catalog_cards_are_optional_disclosed_and_keep_the_dock_description_free() {
    let mut app = scene_app(1920, 1080, UiScaleMode::Auto);
    assert!(find_named(app.world_mut(), "Selected Skill").is_none());
    assert!(find_named(app.world_mut(), "Target Legality").is_none());
    let source = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .as_ref()
        .and_then(|snapshot| snapshot.active_actor)
        .expect("hero");
    for skills in SkillId::ALL.chunks(MAX_EQUIPPED_ABILITIES) {
        app.world_mut()
            .resource_mut::<LabyrinthView>()
            .combat
            .as_mut()
            .expect("combat")
            .actors
            .iter_mut()
            .find(|actor| actor.id == source)
            .expect("actor")
            .abilities = AbilityLoadout::new(skills.iter().copied()).expect("catalog loadout");
        run_frames(&mut app, 5);
        for (index, skill) in skills.iter().enumerate() {
            let control = find_named(app.world_mut(), &format!("Skill {index}")).expect("ability");
            assert!(focus_action(app.world_mut(), control));
            run_frames(&mut app, 8);
            let title = find_named(app.world_mut(), "Tooltip Title").expect("delayed card");
            assert_eq!(
                app.world().get::<Text>(title).expect("title").0,
                skill_definition(*skill).name
            );
            let titles = app
                .world_mut()
                .query::<(&Name, &Text)>()
                .iter(app.world())
                .filter(|(name, text)| {
                    name.as_str() == "Tooltip Title" && text.0 == skill_definition(*skill).name
                })
                .count();
            assert_eq!(titles, 1);
        }
    }
    tap_key(&mut app, KeyCode::KeyK);
    run_frames(&mut app, 5);
    assert!(find_named(app.world_mut(), "Skillbook").is_some());
    let selection = app.world().resource::<UiState>().selected;
    let book_link = find_named(
        app.world_mut(),
        &format!(
            "Read {}",
            skill_definition(*SkillId::ALL.last().expect("catalog")).name
        ),
    )
    .expect("book entry");
    assert!(focus_action(app.world_mut(), book_link));
    tap_key(&mut app, KeyCode::Enter);
    run_frames(&mut app, 4);
    assert!(app
        .world()
        .resource::<bevy_game_ui::UiTooltipState>()
        .is_pinned());
    assert_eq!(app.world().resource::<UiState>().selected, selection);
    assert_eq!(app.world().resource::<CapturedCombatIntents>().0, 0);
}

#[test]
fn tooltip_is_never_visible_at_unplaced_geometry() {
    use bevy_game_ui::{UiTooltipCatalog, UiTooltipContent, UiTooltipRequest, UiTooltipSubject};

    let mut app = scene_app(1280, 720, UiScaleMode::Auto);
    let subject = UiTooltipSubject("placement-regression".to_owned());
    for body in [
        "Short explanation".to_owned(),
        "Long explanation. ".repeat(150),
    ] {
        app.world_mut().resource_mut::<UiTooltipCatalog>().0.insert(
            subject.clone(),
            UiTooltipContent {
                title: "Placement regression".to_owned(),
                body,
                ..default()
            },
        );
        app.world_mut()
            .write_message(UiTooltipRequest::Open(subject.clone()));
        let mut visible_frames = 0;
        // Inspect every frame, including creation and content replacement, not
        // just a screenshot after fixed-frame settling has hidden the defect.
        for frame in 0..12 {
            app.update();
            let card = find_named(app.world_mut(), "Tooltip Card 0").expect("card created");
            let visible = app
                .world()
                .get::<InheritedVisibility>(card)
                .expect("visibility")
                .get();
            if !visible {
                continue;
            }
            visible_frames += 1;
            let node = app.world().get::<ComputedNode>(card).expect("layout");
            let transform = app
                .world()
                .get::<UiGlobalTransform>(card)
                .expect("transform");
            let rect = Rect::from_center_size(
                transform.translation * node.inverse_scale_factor,
                node.size() * node.inverse_scale_factor,
            );
            let bounds = app
                .world_mut()
                .query::<&bevy_game_ui::UiTooltipBounds>()
                .single(app.world())
                .expect("safe area")
                .0;
            assert!(
                rect.min.cmpge(bounds.min).all() && rect.max.cmple(bounds.max + Vec2::ONE).all(),
                "visible before placement on frame {frame}: {rect:?}, safe area {bounds:?}"
            );
            let style = app.world().get::<Node>(card).expect("style");
            let (left, top) = match (style.left, style.top) {
                (Val::Px(left), Val::Px(top)) => Some((left, top)),
                _ => None,
            }
            .expect("visible card has a position");
            assert!(rect.min.distance(Vec2::new(left, top)) < 1.0,
                "visible geometry must have consumed placement: frame {frame}, {rect:?}, {left}, {top}");
        }
        assert!(visible_frames > 0, "card must eventually appear");
    }
}

#[test]
fn opening_a_link_keeps_the_parent_tooltip_visible() {
    let mut app = scene_app(1280, 720, UiScaleMode::Auto);
    let source = find_named(app.world_mut(), "Skill 0").expect("ability");
    assert!(focus_action(app.world_mut(), source));
    run_frames(&mut app, 10);
    let card = find_named(app.world_mut(), "Tooltip Card 0").expect("root");
    let before = *app
        .world()
        .get::<UiGlobalTransform>(card)
        .expect("transform");
    let link = find_named(app.world_mut(), "Tooltip Formation ranks ›").expect("link");
    // This test concerns layout continuity at the activation boundary; pointer
    // routing is exercised separately by the native-input fixture.
    app.world_mut()
        .write_message(bevy_game_ui::UiActivated { entity: link });
    for frame in 0..6 {
        app.update();
        let card = find_named(app.world_mut(), "Tooltip Card 0").expect("root");
        assert!(
            app.world()
                .get::<InheritedVisibility>(card)
                .expect("visibility")
                .get(),
            "unchanged parent disappeared on frame {frame}"
        );
        assert_eq!(
            *app.world()
                .get::<UiGlobalTransform>(card)
                .expect("transform"),
            before
        );
    }
    let child = find_named(app.world_mut(), "Tooltip Card 1").expect("linked card");
    assert!(app
        .world()
        .get::<InheritedVisibility>(child)
        .expect("visibility")
        .get());
}

#[test]
fn tooltip_pointer_and_native_wheel_do_not_select_underlying_characters() {
    use bevy_game_ui::{UiTooltipRequest, UiTooltipState};
    for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
        let mut app = scene_app(width, height, UiScaleMode::Auto);
        let expected = geometry(&mut app);
        let snapshot = app
            .world()
            .resource::<LabyrinthView>()
            .combat
            .clone()
            .expect("combat");
        let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
        let source = find_named(app.world_mut(), "Skill 0").expect("ability");
        hover_control(&mut app, source, viewport);
        let card = find_named(app.world_mut(), "Tooltip Card 0").expect("card");
        let rect = visible_control_rect(app.world(), card, viewport).expect("visible card");
        assert!(
            app.world()
                .get::<InheritedVisibility>(card)
                .expect("visibility")
                .get(),
            "tooltip not visible at {width}x{height}: rect={rect:?}, node={:?}",
            app.world().get::<Node>(card)
        );
        let bounds = app
            .world_mut()
            .query::<&bevy_game_ui::UiTooltipBounds>()
            .single(app.world())
            .expect("safe area")
            .0;
        assert!(
            rect.max.y <= bounds.max.y + 1.0,
            "keep HP visible: {rect:?}, {bounds:?}"
        );
        let target = app.world().resource::<UiState>().target;
        native_pointer_click(&mut app, rect.min + Vec2::splat(20.0));
        run_frames(&mut app, 3);
        assert_eq!(app.world().resource::<UiState>().target, target);
        unchanged(&mut app, &expected, &snapshot, "tooltip click");
        app.world_mut().write_message(UiTooltipRequest::Dismiss);
        run_frames(&mut app, 1);
        let wait = find_named(app.world_mut(), "Wait").expect("wait");
        app.world_mut().entity_mut(wait).insert(UiContextHelp {
            title: "Wait".to_owned(),
            body: ["Waiting spends a turn without changing formation."; 128].join("\n"),
        });
        assert!(focus_action(app.world_mut(), wait));
        run_frames(&mut app, 8);
        let card = find_named(app.world_mut(), "Tooltip Card 0").expect("long card");
        let rect = visible_control_rect(app.world(), card, viewport).expect("card bounds");
        app.world_mut().resource_mut::<InputFocus>().clear();
        hover_at(&mut app, rect.center());
        let before = app
            .world()
            .get::<ScrollPosition>(card)
            .map_or(0.0, |position| position.0.y);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .expect("window");
        let event = bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -8.0,
            window,
            phase: bevy::input::touch::TouchPhase::Moved,
        };
        app.world_mut().write_message(event);
        app.world_mut()
            .write_message(bevy::window::WindowEvent::MouseWheel(event));
        run_frames(&mut app, 3);
        let after = app
            .world()
            .get::<ScrollPosition>(card)
            .expect("native scroll")
            .0
            .y;
        assert!(after > before, "native wheel scrolls inspection, not game");
        tap_key(&mut app, KeyCode::End);
        run_frames(&mut app, 3);
        let node = app.world().get::<ComputedNode>(card).expect("card");
        let maximum =
            ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
        assert!(
            (app.world()
                .get::<ScrollPosition>(card)
                .expect("end position")
                .0
                .y
                - maximum)
                .abs()
                < 1.0
        );
        tap_key(&mut app, KeyCode::Home);
        run_frames(&mut app, 3);
        assert_eq!(
            app.world()
                .get::<ScrollPosition>(card)
                .expect("home position")
                .0
                .y,
            0.0
        );
        assert!(!app
            .world()
            .resource::<UiTooltipState>()
            .subjects()
            .is_empty());
        unchanged(&mut app, &expected, &snapshot, "tooltip scroll");
    }
}

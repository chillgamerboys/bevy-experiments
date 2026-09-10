use super::*;

#[test]
fn log_toggle_has_no_tooltip_before_or_after_pointer_and_keyboard_activation() {
    for keyboard in [false, true] {
        let mut app = app(1280, 720, UiScaleMode::Auto);
        let toolbar = find_named(app.world_mut(), "Battle Log Toggle").expect("log toggle");
        let point = visible_control_rect(
            app.world(),
            toolbar,
            Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
        )
        .expect("toolbar geometry")
        .center();
        app.world_mut()
            .resource_mut::<bevy_game_ui::UiTooltipSettings>()
            .show_delay = std::time::Duration::ZERO;
        if keyboard {
            assert!(focus_action(app.world_mut(), toolbar));
        } else {
            overlay_stability::hover_at(&mut app, point);
        }
        run_frames(&mut app, 3);
        assert!(app
            .world()
            .resource::<bevy_game_ui::UiTooltipState>()
            .subjects()
            .is_empty());
        if keyboard {
            tap_key(&mut app, KeyCode::Enter);
        } else {
            overlay_stability::native_pointer_click(&mut app, point);
        }
        run_frames(&mut app, 30);
        assert!(find_named(app.world_mut(), "Combat History").is_some());
        assert!(app
            .world()
            .resource::<bevy_game_ui::UiTooltipState>()
            .subjects()
            .is_empty());
        assert!(find_named(app.world_mut(), "Tooltip Card 0").is_none());
    }
}

#[test]
fn hidden_compact_and_history_have_distinct_content_and_input_surfaces() {
    for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
        let mut app = app(width, height, UiScaleMode::Auto);
        assert!(find_named(app.world_mut(), "Combat History").is_none());
        assert!(find_named(app.world_mut(), "Inspector Toggle").is_none());
        assert!(find_named(app.world_mut(), "Timeline Toggle").is_none());
        assert!(find_named(app.world_mut(), "Battle Detail Drawer").is_none());
        let before = app.world().resource::<LabyrinthView>().combat.clone();
        let mut kinds = Vec::new();
        for _ in 0..3 {
            kinds.push(labyrinth_rules::CombatEventKind::Action {
                actor: ActorId(105),
                action: CombatAction::Wait,
            });
            kinds.push(labyrinth_rules::CombatEventKind::Damage {
                source: ActorId(105),
                target: ActorId(4),
                amount: 4,
                kind: labyrinth_rules::DamageKind::Direct,
            });
        }
        app.world_mut().resource_mut::<LabyrinthView>().events = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| crate::view::PresentedEvent {
                id: i as u64 + 1,
                event: labyrinth_rules::CombatEvent {
                    id: i as u64 + 1,
                    kind,
                },
            })
            .collect();
        run_frames(&mut app, 3);
        assert!(
            find_named(app.world_mut(), "Combat History").is_none(),
            "incoming events never reopen the log"
        );
        let toolbar = find_named(app.world_mut(), "Battle Log Toggle").expect("open history");
        assert!(click_action(&mut app, toolbar));
        run_frames(&mut app, 3);
        assert!(find_named(app.world_mut(), "History Latest").is_some());
        assert!(find_named(app.world_mut(), "History Expand 1").is_some());
        let compact = find_named(app.world_mut(), "History Toggle").expect("compact");
        assert!(click_action(&mut app, compact));
        run_frames(&mut app, 3);
        assert_eq!(app.world().resource::<UiState>().log_mode, LogMode::Compact);
        assert!(find_named(app.world_mut(), "History Latest").is_none());
        assert!(find_named(app.world_mut(), "History Expand 1").is_none());
        let summaries = app
            .world_mut()
            .query::<(&Name, &Text)>()
            .iter(app.world())
            .filter(|(name, _)| name.as_str() == "History Summary")
            .map(|(_, text)| text.0.clone())
            .collect::<Vec<_>>();
        assert_eq!(summaries.len(), 2);
        assert!(summaries.iter().all(|line| line.contains("−4 HP")));
        let texts = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .filter(|(_, name)| name.as_str() == "History Summary")
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();
        for entity in texts {
            let node = app
                .world()
                .get::<ComputedNode>(entity)
                .expect("measured summary");
            let visible = visible_control_rect(
                app.world(),
                entity,
                Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
            )
            .expect("visible summary");
            assert!(
                visible.height() + 0.5 >= node.size().y * node.inverse_scale_factor,
                "compact summary clipped at {width}x{height}"
            );
        }
        let hide = find_named(app.world_mut(), "History Hide").expect("hide compact");
        assert!(click_action(&mut app, hide));
        run_frames(&mut app, 3);
        assert!(find_named(app.world_mut(), "Combat History").is_none());
        assert!(find_named(app.world_mut(), "History Hide").is_none());
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(toolbar));
        assert_eq!(app.world().resource::<LabyrinthView>().events.len(), 6);
        assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
    }
}

#[test]
fn expanding_an_action_and_its_ability_is_inspection_not_gameplay() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let kinds = [
        labyrinth_rules::CombatEventKind::Action {
            actor: ActorId(105),
            action: CombatAction::Skill {
                skill: SkillId::HollowBolt,
                target: ActorId(4),
            },
        },
        labyrinth_rules::CombatEventKind::Damage {
            source: ActorId(105),
            target: ActorId(4),
            amount: 4,
            kind: labyrinth_rules::DamageKind::Direct,
        },
    ];
    app.world_mut().resource_mut::<LabyrinthView>().events = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| crate::view::PresentedEvent {
            id: index as u64 + 1,
            event: labyrinth_rules::CombatEvent {
                id: index as u64 + 1,
                kind,
            },
        })
        .collect();
    run_frames(&mut app, 3);
    let actor = find_named(app.world_mut(), "Actor 105").expect("target");
    let before = app.world().get::<UiGlobalTransform>(actor).copied();
    let mut intents = bevy::ecs::message::MessageCursor::<LabyrinthIntent>::default();
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 3);
    let expand = find_named(app.world_mut(), "History Expand 1").expect("expand outcome");
    assert!(click_action(&mut app, expand));
    run_frames(&mut app, 3);
    let ability = find_named(app.world_mut(), "History Ability 1").expect("disclosed ability");
    let subject = app
        .world()
        .get::<bevy_game_ui::UiTooltipOpen>(ability)
        .expect("ability link")
        .0
        .clone();
    assert!(click_action(&mut app, ability));
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .resource::<bevy_game_ui::UiTooltipState>()
        .is_pinned());
    assert_eq!(
        app.world()
            .resource::<bevy_game_ui::UiTooltipState>()
            .subjects(),
        &[subject]
    );
    assert_eq!(app.world().get::<UiGlobalTransform>(actor).copied(), before);
    assert!(activation_eligible(app.world_mut(), actor));
    assert_eq!(
        intents
            .read(app.world().resource::<Messages<LabyrinthIntent>>())
            .count(),
        0
    );
    // Disclosure revocation removes both historical detail and its open card.
    app.world_mut()
        .resource_mut::<crate::presentation::CombatDisclosure>()
        .actors
        .insert(
            ActorId(105),
            crate::presentation::ActorDisclosure {
                health: false,
                statuses: false,
                details: false,
            },
        );
    run_frames(&mut app, 3);
    assert!(find_named(app.world_mut(), "History Ability 1").is_none());
    assert!(app
        .world()
        .resource::<bevy_game_ui::UiTooltipState>()
        .subjects()
        .is_empty());
}

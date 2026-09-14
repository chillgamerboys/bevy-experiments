//! Draft and production-plugin structural input evidence; no rendered/desktop/IME claim.

use super::*;
use bevy::text::{EditableText, TextEdit};
use bevy_gamekit::testing::{find_named, focus_action, run_frames, TestAppBuilder};
use bevy_gamekit::ui::UiDisabled;
use labyrinth_rules::scenario::StockScenario;

#[expect(
    clippy::panic,
    reason = "Unexpected fixture variants must fail this regression test."
)]
fn fixture() -> LabyrinthView {
    let catalog = ContentCatalog::builtin().expect("builtin catalog");
    let scenario =
        Scenario::stock(StockScenario::WeaponComparison, 42, &catalog).expect("stock scenario");
    let company = scenario
        .heroes
        .iter()
        .map(|actor| crate::session::CompanyMember {
            actor: actor.id,
            hero: match actor.actor.appearance {
                labyrinth_rules::ActorKind::Hero(hero) => hero,
                _ => panic!("hero stock appearance"),
            },
            resolved_build: actor.actor.resolve(&catalog).expect("resolved build"),
            owner: 0,
        })
        .collect();
    LabyrinthView {
        mode: ViewMode::Lobby,
        local: true,
        host: true,
        admitted: true,
        player: Some(0),
        setup_revision: 8,
        revision: 20,
        scenario: Some(scenario),
        catalog: Some(catalog),
        company,
        players: vec![crate::view::PlayerView {
            slot: 0,
            actors: (1..=6).map(ActorId).collect(),
            name: "Host".into(),
            occupied: true,
            connected: true,
            ready: false,
        }],
        ..default()
    }
}

fn first_actor(view: &LabyrinthView) -> &ScenarioActor {
    view.scenario
        .as_ref()
        .expect("scenario")
        .heroes
        .first()
        .expect("first hero")
}

fn edit(view: &LabyrinthView) -> UiState {
    let mut ui = UiState::default();
    assert!(action(view, &mut ui, SetupAction::Edit(first_actor(view).id)).is_none());
    assert!(ui.editor.is_some());
    ui
}

fn id(value: &str) -> ContentId {
    ContentId::new(value).expect("constant ID")
}

#[test]
fn ownership_changes_refresh_mounted_parameter_controls_without_losing_draft_or_caret() {
    let mut app = editor_app();
    app.world_mut().resource_mut::<LabyrinthView>().host = false;
    run_frames(&mut app, 2);
    let field = name_field(app.world_mut());
    let status = find_named(app.world_mut(), "Starting Bleed").expect("status toggle");
    let footprint = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| {
            matches!(field, Field::Build(BuildField::Footprint)).then_some(entity)
        })
        .expect("footprint field");
    assert!(app.world().get::<UiDisabled>(footprint).is_some());
    assert!(focus_action(app.world_mut(), field));
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(field)
            .expect("text");
        text.queue_edit(TextEdit::TextEnd(false));
        text.queue_edit(TextEdit::Insert(" retained".into()));
        text.queue_edit(TextEdit::Left(false));
    }
    run_frames(&mut app, 3);
    let draft = field_text(app.world(), field);
    let caret = app
        .world()
        .get::<EditableText>(field)
        .expect("text")
        .editor()
        .raw_selection()
        .focus()
        .index();
    for owner in [1, 0] {
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            view.company.first_mut().expect("character").owner = owner;
            view.setup_revision += 1;
            view.assignment_revision += 1;
        }
        run_frames(&mut app, 3);
        assert_eq!(name_field(app.world_mut()), field, "field remains mounted");
        assert_eq!(find_named(app.world_mut(), "Starting Bleed"), Some(status));
        assert_eq!(field_text(app.world(), field), draft);
        assert_eq!(
            app.world()
                .resource::<UiState>()
                .editor
                .as_ref()
                .expect("draft")
                .name,
            draft
        );
        assert_eq!(
            app.world()
                .get::<EditableText>(field)
                .expect("text")
                .editor()
                .raw_selection()
                .focus()
                .index(),
            caret
        );
        for entity in [field, status] {
            assert_eq!(app.world().get::<UiDisabled>(entity).is_some(), owner != 0);
        }
        assert!(
            app.world().get::<UiDisabled>(footprint).is_some(),
            "guest footprint stays host-only"
        );
        assert_eq!(focus_action(app.world_mut(), field), owner == 0);
        let save = find_named(app.world_mut(), "Apply Build").expect("save");
        assert!(
            app.world().get::<UiDisabled>(save).is_some(),
            "stale draft still needs reload"
        );
    }
}

#[test]
fn ownership_changes_refresh_inspected_choices_without_enabling_unavailable_choices() {
    let mut app = editor_app();
    app.world_mut().resource_mut::<LabyrinthView>().host = false;
    let actor = first_actor(app.world().resource::<LabyrinthView>()).id;
    super::super::apply_action(
        app.world_mut(),
        Action::Setup(SetupAction::Category(Category::Equipment)),
    );
    for (weapon, unavailable) in [("dagger", false), ("greatsword", true)] {
        super::super::apply_action(
            app.world_mut(),
            Action::Setup(SetupAction::Inspect(
                actor,
                Selection::Weapon(Some(id(weapon))),
            )),
        );
        run_frames(&mut app, 3);
        let choice = find_named(app.world_mut(), "Apply Inspected Choice").expect("choice");
        for owner in [1, 0] {
            {
                let mut view = app.world_mut().resource_mut::<LabyrinthView>();
                view.company.first_mut().expect("character").owner = owner;
                view.setup_revision += 1;
                view.assignment_revision += 1;
            }
            run_frames(&mut app, 3);
            assert_eq!(
                find_named(app.world_mut(), "Apply Inspected Choice"),
                Some(choice)
            );
            assert_eq!(
                app.world().get::<UiDisabled>(choice).is_some(),
                owner != 0 || unavailable
            );
            assert_eq!(
                focus_action(app.world_mut(), choice),
                owner == 0 && !unavailable
            );
        }
    }
}

#[test]
fn invalid_draft_numbers_do_not_submit_or_mutate_authoritative_scenario() {
    let view = fixture();
    let authoritative = view.scenario.clone();
    for (field, value) in [
        (BuildField::MaxHp, "many"),
        (BuildField::MaxHp, "0"),
        (BuildField::Speed, "-1"),
        (BuildField::Footprint, "256"),
        (BuildField::Footprint, "0"),
        (BuildField::StartingHp, "65536"),
        (BuildField::StartingHp, "41"),
        (BuildField::Footprint, "2"),
        (BuildField::StartingHp, "-1"),
    ] {
        let mut ui = edit(&view);
        let initial = ui.editor.as_ref().expect("editor").draft.clone();
        change(&mut ui, field, value);
        assert!(
            action(&view, &mut ui, SetupAction::Save).is_none(),
            "{field:?}: {value}"
        );
        let editor = ui.editor.as_ref().expect("invalid draft retained");
        assert!(editor.error.is_some(), "{field:?}: {value}");
        assert!(!editor.pending_save);
        assert_eq!(editor.draft, initial);
        assert_eq!(view.scenario, authoritative);
    }
}

#[test]
#[expect(
    clippy::panic,
    reason = "Unexpected fixture variants must fail this regression test."
)]
fn composed_submission_and_saved_scenario_preserve_exact_build_and_starting_fields() {
    let view = fixture();
    let authoritative = view.scenario.clone();
    let mut ui = edit(&view);
    let original = first_actor(&view).clone();
    for (field, value) in [
        (BuildField::Name, "Ada the Duelist"),
        (BuildField::MaxHp, "44"),
        (BuildField::Speed, "7"),
        (BuildField::Footprint, "1"),
        (BuildField::StartingHp, "31"),
    ] {
        change(&mut ui, field, value);
    }
    for selected in [
        SetupAction::Weapon(Some(id("dagger"))),
        SetupAction::Skill(id("rally")),
        SetupAction::Ability(id("resilient")),
        SetupAction::Ability(id("duelist_dagger_power")),
        SetupAction::Status(labyrinth_rules::StatusKind::Haste),
    ] {
        assert!(action(&view, &mut ui, selected).is_none());
    }
    let Some(LabyrinthIntent::CustomizeActor {
        actor,
        expected_revision,
    }) = action(&view, &mut ui, SetupAction::Save)
    else {
        panic!("valid actor submission");
    };
    assert_eq!(expected_revision, view.setup_revision);
    assert_eq!(actor.id, original.id);
    assert_eq!(actor.controller, original.controller);
    assert_eq!(actor.actor.appearance, original.actor.appearance);
    assert_eq!(actor.actor.name, "Ada the Duelist");
    assert_eq!(
        (
            actor.actor.max_hp,
            actor.actor.base_speed,
            actor.actor.footprint
        ),
        (44, 7, 1)
    );
    assert_eq!(actor.starting_hp, Some(31));
    assert_eq!(actor.actor.build.weapon, Some(id("dagger")));
    let mut expected_skills = original.actor.build.skills.clone();
    expected_skills.push(SkillGrant {
        skill: id("rally"),
        provenance: id("skills"),
    });
    assert_eq!(actor.actor.build.skills, expected_skills);
    assert_eq!(
        actor.actor.build.abilities,
        vec![id("resilient"), id("duelist_dagger_power")]
    );
    assert_eq!(
        actor.starting_statuses,
        vec![labyrinth_rules::scenario::StartingStatus {
            kind: labyrinth_rules::StatusKind::Haste,
            source: None,
            remaining: None,
        }]
    );
    let mut saved = view.scenario.clone().expect("scenario");
    *saved.heroes.first_mut().expect("first hero") = actor;
    let json = saved.to_json().expect("save editor-produced scenario");
    assert_eq!(
        Scenario::from_json(&json, view.catalog.as_ref().expect("catalog")).expect("reload"),
        saved
    );
    assert_eq!(view.scenario, authoritative);
    assert!(
        ui.editor
            .as_ref()
            .expect("waiting for authority")
            .pending_save
    );
}

#[test]
#[expect(
    clippy::panic,
    reason = "Unexpected fixture variants must fail this regression test."
)]
fn empty_starting_hp_is_full_and_toggle_removal_preserves_other_grant_sources() {
    let view = fixture();
    let mut ui = edit(&view);
    change(&mut ui, BuildField::StartingHp, "   ");
    let original = first_actor(&view).actor.build.skills.clone();
    action(&view, &mut ui, SetupAction::Skill(id("rally")));
    action(&view, &mut ui, SetupAction::Skill(id("rally")));
    action(&view, &mut ui, SetupAction::Ability(id("resilient")));
    action(&view, &mut ui, SetupAction::Ability(id("resilient")));
    action(&view, &mut ui, SetupAction::Weapon(None));
    let Some(LabyrinthIntent::CustomizeActor { actor, .. }) =
        action(&view, &mut ui, SetupAction::Save)
    else {
        panic!("valid unarmed build");
    };
    assert_eq!(actor.starting_hp, None);
    assert_eq!(actor.actor.build.weapon, None);
    assert_eq!(actor.actor.build.skills, original);
    assert!(actor.actor.build.abilities.is_empty());
}

#[test]
fn stale_or_reassigned_draft_cannot_submit_against_a_new_configuration() {
    let mut view = fixture();
    let mut ui = edit(&view);
    change(&mut ui, BuildField::Name, "Unsubmitted draft");
    let revision = view.setup_revision;
    view.setup_revision += 1;
    assert!(action(&view, &mut ui, SetupAction::Save).is_none());
    let editor = ui.editor.as_ref().expect("draft retained");
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.name, "Unsubmitted draft");
    assert!(editor.error.is_some());
    assert!(!editor.pending_save);

    view.local = false;
    view.host = false;
    view.player = Some(1);
    view.company
        .first_mut()
        .expect("first company member")
        .owner = 1;
    let mut ui = edit(&view);
    view.company
        .first_mut()
        .expect("first company member")
        .owner = 2;
    assert!(action(&view, &mut ui, SetupAction::Save).is_none());
    assert!(ui.editor.as_ref().expect("draft retained").error.is_some());
}

fn editor_app() -> App {
    let mut builder = TestAppBuilder::new().with_ui(1280, 900);
    builder
        .app_mut()
        .insert_resource(fixture())
        .add_plugins(super::super::LabyrinthUiPlugin);
    let mut app = builder.build();
    run_frames(&mut app, 4);
    let actor = first_actor(app.world().resource::<LabyrinthView>()).id;
    super::super::apply_action(app.world_mut(), Action::Setup(SetupAction::Edit(actor)));
    super::super::apply_action(
        app.world_mut(),
        Action::Setup(SetupAction::Category(Category::Parameters)),
    );
    run_frames(&mut app, 4);
    app
}

fn name_field(world: &mut World) -> Entity {
    world
        .query::<(Entity, &Field)>()
        .iter(world)
        .find_map(|(entity, field)| {
            matches!(field, Field::Build(BuildField::Name)).then_some(entity)
        })
        .expect("mounted name field")
}

fn field_text(world: &World, entity: Entity) -> String {
    world
        .get::<EditableText>(entity)
        .expect("native text")
        .value()
        .into_iter()
        .collect()
}

#[test]
fn participant_refresh_keeps_native_field_text_focus_and_caret_while_conflict_disables_apply() {
    let mut app = editor_app();
    let field = name_field(app.world_mut());
    assert!(focus_action(app.world_mut(), field));
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(field)
            .expect("native text");
        text.queue_edit(TextEdit::TextEnd(false));
        text.queue_edit(TextEdit::Insert(" draft".into()));
        text.queue_edit(TextEdit::Left(false));
    }
    run_frames(&mut app, 3);
    let draft = field_text(app.world(), field);
    let caret = app
        .world()
        .get::<EditableText>(field)
        .expect("native text")
        .editor()
        .raw_selection()
        .focus()
        .index();
    assert!(draft.ends_with(" draft"));
    assert!(caret < draft.len(), "cursor is intentionally inside text");
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.revision += 1;
        view.players.push(crate::view::PlayerView {
            slot: 1,
            actors: Vec::new(),
            name: "Spectator".into(),
            occupied: true,
            connected: true,
            ready: false,
        });
    }
    run_frames(&mut app, 3);
    assert_eq!(name_field(app.world_mut()), field);
    assert_eq!(field_text(app.world(), field), draft);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    assert_eq!(
        app.world()
            .get::<EditableText>(field)
            .expect("native text")
            .editor()
            .raw_selection()
            .focus()
            .index(),
        caret
    );
    assert_eq!(
        app.world()
            .resource::<UiState>()
            .editor
            .as_ref()
            .expect("draft")
            .name,
        draft
    );

    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .setup_revision += 1;
    run_frames(&mut app, 2);
    assert_eq!(name_field(app.world_mut()), field);
    assert_eq!(field_text(app.world(), field), draft);
    let apply = find_named(app.world_mut(), "Apply Build").expect("apply control");
    assert!(app.world().get::<UiDisabled>(apply).is_some());
    assert!(!focus_action(app.world_mut(), apply));
    let notice = find_named(app.world_mut(), "Build Notice").expect("notice");
    assert!(app
        .world()
        .get::<Text>(notice)
        .expect("notice text")
        .0
        .contains("Reload"));
}

#[test]
#[expect(
    clippy::panic,
    reason = "Unexpected fixture variants must fail this regression test."
)]
fn pending_save_requires_matching_authoritative_ack_and_reload_or_close_retires_old_fields() {
    let mut app = editor_app();
    let initial = name_field(app.world_mut());
    let view = app.world().resource::<LabyrinthView>().clone();
    app.world_mut()
        .get_mut::<EditableText>(initial)
        .expect("native text")
        .editor_mut()
        .set_text("Accepted draft");
    run_frames(&mut app, 2);
    let submitted = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let Some(LabyrinthIntent::CustomizeActor { actor, .. }) =
            action(&view, &mut ui, SetupAction::Save)
        else {
            panic!("valid draft");
        };
        actor
    };
    // A participant-only projection and a server rejection do not discard the draft.
    app.world_mut().resource_mut::<LabyrinthView>().notice =
        Some("Build rejected by authority".into());
    run_frames(&mut app, 2);
    assert_eq!(name_field(app.world_mut()), initial);
    assert!(app.world().resource::<UiState>().editor.is_some());
    {
        let mut current = app.world_mut().resource_mut::<LabyrinthView>();
        current.setup_revision += 1;
        current.revision += 1;
        *current
            .scenario
            .as_mut()
            .expect("scenario")
            .heroes
            .first_mut()
            .expect("first hero") = submitted;
        current.notice = None;
    }
    run_frames(&mut app, 2);
    assert!(app.world().resource::<UiState>().editor.is_none());
    assert!(app.world().get_entity(initial).is_err());

    super::super::apply_action(
        app.world_mut(),
        Action::Setup(SetupAction::Edit(first_actor(&view).id)),
    );
    super::super::apply_action(
        app.world_mut(),
        Action::Setup(SetupAction::Category(Category::Parameters)),
    );
    run_frames(&mut app, 2);
    let prior = name_field(app.world_mut());
    {
        let mut current = app.world_mut().resource_mut::<LabyrinthView>();
        current.setup_revision += 1;
        current
            .scenario
            .as_mut()
            .expect("scenario")
            .heroes
            .first_mut()
            .expect("first hero")
            .actor
            .name = "Remote update".into();
    }
    super::super::apply_action(app.world_mut(), Action::Setup(SetupAction::Reload));
    run_frames(&mut app, 2);
    let replacement = name_field(app.world_mut());
    assert_ne!(replacement, prior);
    assert!(app.world().get_entity(prior).is_err());
    assert_eq!(field_text(app.world(), replacement), "Remote update");
    let editor = app
        .world()
        .resource::<UiState>()
        .editor
        .as_ref()
        .expect("reloaded");
    assert!(!editor.pending_save);
    assert!(editor.error.is_none());
    super::super::apply_action(app.world_mut(), Action::Setup(SetupAction::Cancel));
    run_frames(&mut app, 2);
    assert!(app.world().get_entity(replacement).is_err());
    assert!(app.world().resource::<UiState>().editor.is_none());
}

#[test]
fn correction_clears_local_error_and_new_edits_cancel_pending_close() {
    let view = fixture();
    let mut ui = edit(&view);
    change(&mut ui, BuildField::MaxHp, "not a number");
    assert!(action(&view, &mut ui, SetupAction::Save).is_none());
    assert!(ui.editor.as_ref().expect("editor").error.is_some());
    change(&mut ui, BuildField::MaxHp, "44");
    assert!(ui.editor.as_ref().expect("editor").error.is_none());
    assert!(action(&view, &mut ui, SetupAction::Save).is_some());
    assert!(ui.editor.as_ref().expect("editor").pending_save);
    change(&mut ui, BuildField::Name, "Newer local draft");
    assert!(!ui.editor.as_ref().expect("editor").pending_save);
    assert_eq!(
        ui.editor.as_ref().expect("editor").name,
        "Newer local draft"
    );
    action(&view, &mut ui, SetupAction::Weapon(Some(id("dagger"))));
    assert!(!ui.editor.as_ref().expect("editor").pending_save);
    assert!(ui.editor.as_ref().expect("editor").error.is_none());
}

#[test]
fn enemy_starting_down_or_oversized_formation_is_rejected_without_scenario_mutation() {
    let view = fixture();
    let authoritative = view.scenario.clone();
    let enemy = view
        .scenario
        .as_ref()
        .expect("scenario")
        .enemies
        .first()
        .expect("first enemy")
        .id;
    let mut ui = UiState::default();
    action(&view, &mut ui, SetupAction::Edit(enemy));
    change(&mut ui, BuildField::StartingHp, "0");
    assert!(action(&view, &mut ui, SetupAction::Save).is_none());
    assert!(ui.editor.as_ref().expect("draft retained").error.is_some());
    assert_eq!(view.scenario, authoritative);
}

#[test]
fn no_op_save_closes_on_new_authoritative_projection_without_setup_revision_change() {
    let mut app = editor_app();
    let field = name_field(app.world_mut());
    let view = app.world().resource::<LabyrinthView>().clone();
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        assert!(action(&view, &mut ui, SetupAction::Save).is_some());
    }
    run_frames(&mut app, 2);
    assert!(app.world().resource::<UiState>().editor.is_some());
    app.world_mut().resource_mut::<LabyrinthView>().revision += 1;
    run_frames(&mut app, 2);
    assert_eq!(
        app.world().resource::<LabyrinthView>().setup_revision,
        view.setup_revision
    );
    assert!(app.world().resource::<UiState>().editor.is_none());
    assert!(app.world().get_entity(field).is_err());
}

#[test]
fn leaving_lobby_or_losing_admission_retires_editor_entities() {
    for admitted in [true, false] {
        let mut app = editor_app();
        let field = name_field(app.world_mut());
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            if admitted {
                view.mode = ViewMode::Menu;
            } else {
                view.admitted = false;
            }
        }
        run_frames(&mut app, 2);
        assert!(app.world().resource::<UiState>().editor.is_none());
        assert!(app.world().get_entity(field).is_err());
    }
}

#[test]
fn seed_field_tracks_loaded_configuration_without_losing_an_unapplied_draft() {
    let mut builder = TestAppBuilder::new().with_ui(1280, 900);
    let mut view = fixture();
    view.scenario.as_mut().expect("scenario").seed = 91;
    builder
        .app_mut()
        .insert_resource(view)
        .add_plugins(super::super::LabyrinthUiPlugin);
    let mut app = builder.build();
    run_frames(&mut app, 4);
    super::super::apply_action(app.world_mut(), Action::LobbyPage(2));
    run_frames(&mut app, 3);
    let field = find_named(app.world_mut(), "Scenario seed").expect("seed input");
    assert_eq!(field_text(app.world(), field), "91");
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(field)
            .expect("text");
        text.queue_edit(TextEdit::SelectAll);
        text.queue_edit(TextEdit::Insert("123".into()));
    }
    run_frames(&mut app, 2);
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .players
        .first_mut()
        .expect("host")
        .ready = true;
    run_frames(&mut app, 3);
    let field = find_named(app.world_mut(), "Scenario seed").expect("rebuilt input");
    assert_eq!(field_text(app.world(), field), "123");
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .scenario
        .as_mut()
        .expect("scenario")
        .seed = 456;
    run_frames(&mut app, 3);
    let field = find_named(app.world_mut(), "Scenario seed").expect("loaded input");
    assert_eq!(field_text(app.world(), field), "456");
}

#[test]
fn editor_escape_closes_locally_without_opening_background_menus() {
    let mut app = editor_app();
    let original = app.world().resource::<LabyrinthView>().scenario.clone();
    let field = name_field(app.world_mut());
    assert!(focus_action(app.world_mut(), field));
    bevy_gamekit::testing::tap_key(&mut app, KeyCode::KeyK);
    assert!(!app.world().resource::<UiState>().show_skillbook);
    bevy_gamekit::testing::tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 2);
    let ui = app.world().resource::<UiState>();
    assert!(ui.editor.is_none());
    assert!(!ui.menus.is_open());
    assert!(app.world().get_entity(field).is_err());
    assert_eq!(app.world().resource::<LabyrinthView>().scenario, original);
}

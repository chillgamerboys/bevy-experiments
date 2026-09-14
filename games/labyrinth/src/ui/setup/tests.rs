//! Resolver decision checks and production native-input messages; no human comprehension claim.
#![expect(
    clippy::panic,
    reason = "Authored fixture invariants and typed-intent assertions"
)]
use super::*;
use bevy_gamekit::testing::{
    find_named, focus_action, run_frames, tap_key, visible_control_rect, TestAppBuilder,
};
use labyrinth_rules::{
    build::CharacterBuild,
    catalog::{AbilityUpgrade, LearnedSkillDefinition, UpgradeOperation},
    scenario::StockScenario,
};
fn id(value: &str) -> ContentId {
    ContentId::new(value).expect("fixture ID")
}
fn fixture() -> LabyrinthView {
    let catalog = ContentCatalog::builtin().expect("catalog");
    let mut scenario =
        Scenario::stock(StockScenario::WeaponComparison, 42, &catalog).expect("scenario");
    scenario
        .heroes
        .first_mut()
        .expect("hero")
        .actor
        .build
        .weapon = Some(id("dagger"));
    let company = scenario
        .heroes
        .iter()
        .map(|a| crate::view::CompanyMember {
            actor: a.id,
            hero: match a.actor.appearance {
                labyrinth_rules::ActorKind::Hero(hero) => hero,
                _ => panic!("hero"),
            },
            abilities: a.actor.resolve(&catalog).expect("build"),
            owner: 0,
        })
        .collect();
    LabyrinthView {
        mode: ViewMode::Lobby,
        local: true,
        host: true,
        admitted: true,
        player: Some(0),
        revision: 20,
        setup_revision: 8,
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
fn edit(view: &LabyrinthView) -> UiState {
    let mut ui = UiState::default();
    action(view, &mut ui, SetupAction::Edit(ActorId(1)));
    ui
}
fn inspect(view: &LabyrinthView, ui: &mut UiState, category: Category, selection: Selection) {
    let who = ui.editor.as_ref().expect("editor").id;
    action(view, ui, SetupAction::Category(category));
    action(view, ui, SetupAction::Inspect(who, selection));
}
fn apply(view: &LabyrinthView, ui: &mut UiState) {
    let editor = ui.editor.as_ref().expect("editor");
    action(
        view,
        ui,
        SetupAction::ApplyInspected(editor.id, editor.selection().expect("selection").clone()),
    );
}
fn build(editor: &ActorEditor, catalog: &ContentCatalog) -> labyrinth_rules::build::ResolvedBuild {
    catalog
        .resolve_build(&editor.draft.actor.build)
        .expect("resolved")
}
#[test]
fn inspection_does_not_edit_and_explicit_equip_preserves_parameter_drafts() {
    let view = fixture();
    let mut ui = edit(&view);
    let before = ui.editor.as_ref().expect("editor").draft.clone();
    change(&mut ui, BuildField::Name, "Aster");
    change(&mut ui, BuildField::MaxHp, "45");
    change(&mut ui, BuildField::Speed, "8");
    inspect(
        &view,
        &mut ui,
        Category::Equipment,
        Selection::Weapon(Some(id("greatsword"))),
    );
    assert_eq!(ui.editor.as_ref().expect("editor").draft, before);
    apply(&view, &mut ui);
    inspect(
        &view,
        &mut ui,
        Category::Learned,
        Selection::Learned(id("assassin_feint_training")),
    );
    let editor = ui.editor.as_ref().expect("editor");
    assert_eq!(
        (&*editor.name, &*editor.max_hp, &*editor.speed),
        ("Aster", "45", "8")
    );
    assert_eq!(editor.draft.actor.build.weapon, Some(id("greatsword")));
    let Some(LabyrinthIntent::CustomizeActor { actor, .. }) =
        action(&view, &mut ui, SetupAction::Save)
    else {
        panic!("valid draft submits")
    };
    assert_eq!(actor.actor.max_hp, 45);
    assert_eq!(actor.actor.base_speed, 8);
    assert_eq!(actor.actor.name, "Aster");
}
#[test]
fn rank_restrictions_are_explained_without_removing_granted_moves() {
    let view = fixture();
    let catalog = view.catalog.as_ref().expect("catalog");
    let mut ui = edit(&view);
    action(&view, &mut ui, SetupAction::Weapon(Some(id("dagger"))));
    let resolved = build(ui.editor.as_ref().expect("editor"), catalog);
    let throw = resolved
        .abilities
        .iter()
        .find(|a| a.definition.id == id("dagger_throw"))
        .expect("throw remains granted");
    let front = details::move_facts(throw, (1, 1), catalog).join("\n");
    assert!(front.contains("Unavailable at current rank 1"));
    assert!(front.contains("Acting ranks: 3, 4, 5, 6"));
    assert!(front.contains("Target ranks: 1, 2, 3, 4, 5, 6"));
    assert!(details::move_facts(throw, (4, 4), catalog)
        .join("\n")
        .contains("current position allows"));
    assert!(
        details::move_facts(throw, (2, 3), catalog)
            .join("\n")
            .contains("current position allows"),
        "kernel allows any occupied rank"
    );
    inspect(
        &view,
        &mut ui,
        Category::Equipment,
        Selection::Weapon(Some(id("greatsword"))),
    );
    let inspected = details::inspection(ui.editor.as_ref().expect("editor"), catalog);
    let cleave = inspected
        .moves
        .iter()
        .find(|a| a.definition.id == id("greatsword_cleave"))
        .expect("cleave");
    let text = details::move_facts(cleave, (4, 4), catalog).join("\n");
    assert!(text.contains("Unavailable at current rank 4"));
    assert!(text.contains("Each distinct occupant is hit once"));
}
#[test]
fn learned_prerequisites_upgrades_and_redundant_grant_removal_use_resolver() {
    let view = fixture();
    let catalog = view.catalog.as_ref().expect("catalog");
    let mut ui = edit(&view);
    ui.editor.as_mut().expect("editor").draft.actor.build = CharacterBuild {
        weapon: Some(id("greatsword")),
        ..default()
    };
    inspect(
        &view,
        &mut ui,
        Category::Learned,
        Selection::Learned(id("duelist_dagger_power")),
    );
    let info = details::inspection(ui.editor.as_ref().expect("editor"), catalog);
    assert!(info
        .facts
        .iter()
        .any(|s| s.contains("Requires Dagger Stab from any grant source")));
    assert_eq!(
        info.changes,
        vec!["Cannot apply this choice: Requires Dagger Stab in the resulting build."]
    );
    assert!(info.apply.expect("action").1);
    let mut renamed = catalog.definition().clone();
    renamed
        .abilities
        .iter_mut()
        .find(|a| a.id == id("dagger_stab"))
        .expect("required move")
        .name = "Needle Jab".into();
    let renamed = ContentCatalog::new(renamed).expect("renamed catalog");
    let renamed_info = details::inspection(ui.editor.as_ref().expect("editor"), &renamed);
    assert_eq!(
        renamed_info.changes,
        vec!["Cannot apply this choice: Requires Needle Jab in the resulting build."]
    );
    assert!(renamed_info.apply.expect("still unavailable").1);
    apply(&view, &mut ui);
    assert!(ui
        .editor
        .as_ref()
        .expect("editor")
        .draft
        .actor
        .build
        .learned_skills
        .is_empty());
    action(&view, &mut ui, SetupAction::Weapon(Some(id("dagger"))));
    apply(&view, &mut ui);
    let improved = build(ui.editor.as_ref().expect("editor"), catalog);
    assert!(improved
        .abilities
        .iter()
        .find(|a| a.definition.id == id("dagger_stab"))
        .expect("stab")
        .definition
        .effects
        .contains(&labyrinth_rules::Effect::Damage(7)));
    action(&view, &mut ui, SetupAction::Innate(id("dagger_stab")));
    inspect(
        &view,
        &mut ui,
        Category::Innate,
        Selection::Innate(id("dagger_stab")),
    );
    let info = details::inspection(ui.editor.as_ref().expect("editor"), catalog);
    assert!(info
        .changes
        .iter()
        .any(|s| s.contains("Retained · Dagger Stab")));
    assert!(!info
        .changes
        .iter()
        .any(|s| s.contains("Removed · Dagger Stab")));
    apply(&view, &mut ui);
    assert!(build(ui.editor.as_ref().expect("editor"), catalog)
        .abilities
        .iter()
        .any(|a| a.definition.id == id("dagger_stab")));
}
#[test]
fn rank_only_upgrade_comparison_names_the_actual_changed_fields() {
    let mut view = fixture();
    let mut definition = view.catalog.as_ref().expect("catalog").definition().clone();
    definition.learned_skills.push(LearnedSkillDefinition {
        id: id("reach_training"),
        name: "Reach training".into(),
        description: "Extend ranks".into(),
        provenance: id("training"),
        grants: vec![],
        upgrades: vec![AbilityUpgrade {
            ability: id("dagger_stab"),
            operations: vec![
                UpgradeOperation::ExtendSourceRanks(48),
                UpgradeOperation::ExtendTargetRanks(60),
            ],
        }],
    });
    view.catalog = Some(ContentCatalog::new(definition).expect("catalog"));
    let mut ui = edit(&view);
    action(&view, &mut ui, SetupAction::Weapon(Some(id("dagger"))));
    inspect(
        &view,
        &mut ui,
        Category::Learned,
        Selection::Learned(id("reach_training")),
    );
    let info = details::inspection(
        ui.editor.as_ref().expect("editor"),
        view.catalog.as_ref().expect("catalog"),
    );
    let changes = info.changes.join("\n");
    assert!(changes.contains("acting ranks 1, 2, 3, 4 → 1, 2, 3, 4, 5, 6"));
    assert!(changes.contains("target ranks 1, 2 → 1, 2, 3, 4, 5, 6"));
    assert!(!changes.contains("5 base damage → 5 base damage"));
}
#[test]
fn switching_and_closing_dirty_character_require_explicit_discard() {
    let view = fixture();
    let mut ui = edit(&view);
    change(&mut ui, BuildField::Name, "Unapplied hero");
    let enemy = view
        .scenario
        .as_ref()
        .expect("scenario")
        .enemies
        .first()
        .expect("enemy")
        .id;
    action(&view, &mut ui, SetupAction::Edit(enemy));
    assert_eq!(ui.editor.as_ref().expect("editor").id, ActorId(1));
    assert!(ui.editor.as_ref().expect("editor").pending_exit.is_some());
    action(&view, &mut ui, SetupAction::KeepEditing);
    assert_eq!(ui.editor.as_ref().expect("editor").name, "Unapplied hero");
    action(&view, &mut ui, SetupAction::Edit(enemy));
    action(&view, &mut ui, SetupAction::ConfirmDiscard);
    assert_eq!(ui.editor.as_ref().expect("same editor for enemy").id, enemy);
    change(&mut ui, BuildField::Name, "Unapplied enemy");
    action(&view, &mut ui, SetupAction::Cancel);
    assert!(ui.editor.is_some());
    action(&view, &mut ui, SetupAction::ConfirmDiscard);
    assert!(ui.editor.is_none());
}
#[test]
fn stale_inspection_controls_cannot_mutate_another_selection_or_character() {
    let view = fixture();
    let mut ui = edit(&view);
    let stale = Selection::Weapon(Some(id("greatsword")));
    inspect(&view, &mut ui, Category::Equipment, stale.clone());
    inspect(
        &view,
        &mut ui,
        Category::Equipment,
        Selection::Weapon(Some(id("spear"))),
    );
    let original = ui.editor.as_ref().expect("editor").draft.clone();
    action(
        &view,
        &mut ui,
        SetupAction::ApplyInspected(ActorId(1), stale.clone()),
    );
    assert_eq!(ui.editor.as_ref().expect("editor").draft, original);
    action(&view, &mut ui, SetupAction::Edit(ActorId(2)));
    let original = ui.editor.as_ref().expect("editor").draft.clone();
    action(
        &view,
        &mut ui,
        SetupAction::Inspect(ActorId(1), stale.clone()),
    );
    action(
        &view,
        &mut ui,
        SetupAction::ApplyInspected(ActorId(1), stale),
    );
    assert_eq!(ui.editor.as_ref().expect("editor").draft, original);
}
fn app(width: u32, height: u32, scale: UiScaleMode) -> App {
    let mut builder = TestAppBuilder::new().with_ui(width, height);
    builder
        .app_mut()
        .insert_resource(fixture())
        .insert_resource(UiScalePreference(scale))
        .add_plugins(LabyrinthUiPlugin);
    let mut app = builder.build();
    run_frames(&mut app, 4);
    super::super::apply_action(
        app.world_mut(),
        Action::Setup(SetupAction::Edit(ActorId(1))),
    );
    run_frames(&mut app, 4);
    app
}
fn activate(app: &mut App, name: &str, pointer: bool) {
    let entity = find_named(app.world_mut(), name).expect(name);
    assert!(
        focus_action(app.world_mut(), entity),
        "cannot focus {name}; size/scale {:?} pointer={pointer}",
        app.world().resource::<ResolvedUiMetrics>()
    );
    run_frames(app, 3);
    if pointer {
        use bevy::input::{mouse::MouseButtonInput, ButtonState};
        let size = app.world().resource::<ResolvedUiMetrics>().logical_size;
        let position =
            visible_control_rect(app.world(), entity, Rect::from_corners(Vec2::ZERO, size))
                .expect("visible pointer control")
                .center();
        let (window_id, mut window) = app
            .world_mut()
            .query::<(Entity, &mut Window)>()
            .single_mut(app.world_mut())
            .expect("window");
        window.set_cursor_position(Some(position));
        run_frames(app, 2);
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(MouseButtonInput {
                button: MouseButton::Left,
                state,
                window: window_id,
            });
            app.update();
        }
    } else {
        tap_key(app, KeyCode::Enter);
    }
    run_frames(app, 4);
}
#[test]
fn native_browse_inspect_equip_and_footer_work_at_supported_sizes_normal_1080() {
    native_browse_inspect_equip_and_footer_work_at_supported_sizes(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn native_browse_inspect_equip_and_footer_work_at_supported_sizes_compatibility() {
    native_browse_inspect_equip_and_footer_work_at_supported_sizes(1280, 720, UiScaleMode::Auto);
    native_browse_inspect_equip_and_footer_work_at_supported_sizes(
        1280,
        720,
        UiScaleMode::Percent200,
    );
    native_browse_inspect_equip_and_footer_work_at_supported_sizes(
        1920,
        1080,
        UiScaleMode::Percent200,
    );
}

fn native_browse_inspect_equip_and_footer_work_at_supported_sizes(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    for pointer in [false, true] {
        let mut app = app(width, height, scale);
        let before = app.world().resource::<LabyrinthView>().scenario.clone();
        let original = app
            .world()
            .resource::<UiState>()
            .editor
            .as_ref()
            .expect("editor")
            .draft
            .clone();
        activate(&mut app, "Weapon greatsword", pointer);
        assert_eq!(
            app.world()
                .resource::<UiState>()
                .editor
                .as_ref()
                .expect("editor")
                .draft,
            original,
            "browsing never equips"
        );
        let title =
            find_named(app.world_mut(), "Inspected Choice Title").expect("persistent details");
        assert_eq!(
            app.world().get::<Text>(title).expect("text").0,
            "Greatsword"
        );
        for name in [
            "Apply Inspected Choice",
            "Apply Build",
            "Reload Build",
            "Close Build",
        ] {
            let entity = find_named(app.world_mut(), name).expect("footer or explicit choice");
            let rect = visible_control_rect(
                app.world(),
                entity,
                Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
            )
            .expect("reachable without browsing scroll");
            assert!(
                rect.width() >= 43.5 && rect.height() >= 43.5,
                "{name} {width} {scale:?}: {rect:?}"
            );
        }
        activate(&mut app, "Apply Inspected Choice", pointer);
        assert_eq!(
            app.world()
                .resource::<UiState>()
                .editor
                .as_ref()
                .expect("editor")
                .draft
                .actor
                .build
                .weapon,
            Some(id("greatsword"))
        );
        assert_eq!(app.world().resource::<LabyrinthView>().scenario, before);
        activate(&mut app, "Close Build", pointer);
        assert!(find_named(app.world_mut(), "Discard Confirmation").is_some());
        activate(&mut app, "Keep Editing", pointer);
        assert_eq!(
            app.world()
                .resource::<UiState>()
                .editor
                .as_ref()
                .expect("draft retained")
                .draft
                .actor
                .build
                .weapon,
            Some(id("greatsword"))
        );
    }
}
#[test]
fn compact_back_retains_browser_position_and_parameter_refresh_preserves_field_focus_compatibility()
{
    use bevy::text::EditableText;
    let mut app = app(1280, 720, UiScaleMode::Percent200);
    activate(&mut app, "Category Innate", false);
    let last = app
        .world()
        .resource::<LabyrinthView>()
        .catalog
        .as_ref()
        .expect("catalog")
        .definition()
        .abilities
        .last()
        .expect("ability")
        .id
        .clone();
    activate(&mut app, &format!("Innate {last}"), false);
    activate(&mut app, "Back To Choices", false);
    let browser = find_named(app.world_mut(), "Character Browser").expect("browser");
    assert!(
        app.world()
            .get::<ScrollPosition>(browser)
            .expect("scroll")
            .y
            > 0.
    );
    activate(&mut app, "Category Parameters", false);
    let field = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(e, f)| matches!(f, Field::Build(BuildField::Name)).then_some(e))
        .expect("name");
    assert!(focus_action(app.world_mut(), field));
    let value = app
        .world()
        .get::<EditableText>(field)
        .expect("editable")
        .clone();
    app.world_mut().resource_mut::<LabyrinthView>().revision += 1;
    run_frames(&mut app, 4);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    assert!(app.world().get::<EditableText>(field).is_some());
    let _ = value;
}

#[test]
fn first_detail_fold_shows_effects_and_ranks_and_native_paging_reaches_sources_compatibility() {
    let mut app = app(1280, 720, UiScaleMode::Percent200);
    activate(&mut app, "Weapon greatsword", false);
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(1280., 720.));
    for name in ["Move Tactical Summary", "Move Rank Summary"] {
        let entity = find_named(app.world_mut(), name).expect("decision fact");
        let rect = visible_control_rect(app.world(), entity, viewport)
            .expect("mechanical fact visible without scroll");
        assert!(rect.height() > 30., "{name} visible: {rect:?}");
    }
    let area = find_named(app.world_mut(), "Inspected Choice Scroll").expect("details");
    tap_key(&mut app, KeyCode::End);
    run_frames(&mut app, 3);
    assert!(app.world().get::<ScrollPosition>(area).expect("scrolled").y > 0.);
    tap_key(&mut app, KeyCode::Home);
    run_frames(&mut app, 3);
    assert_eq!(app.world().get::<ScrollPosition>(area).expect("top").y, 0.);
}

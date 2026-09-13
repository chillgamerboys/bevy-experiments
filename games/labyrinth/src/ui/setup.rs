//! Local actor drafts. Authority validates every submitted scenario/build again.

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;

use super::*;
use labyrinth_rules::build::{ActorBuild, InnateGrant};
use labyrinth_rules::catalog::{ContentCatalog, ContentId};
use labyrinth_rules::scenario::{Scenario, ScenarioActor};

#[derive(Debug, Clone, Copy)]
pub(super) enum BuildField {
    Name,
    MaxHp,
    Speed,
    Footprint,
    StartingHp,
}

#[derive(Debug, Clone)]
pub(super) enum SetupAction {
    Edit(ActorId),
    Preset(ContentId),
    Weapon(Option<ContentId>),
    Innate(ContentId),
    Learned(ContentId),
    Status(labyrinth_rules::StatusKind),
    Save,
    Cancel,
    Reload,
    Stock(usize),
    Add(labyrinth_rules::Team),
    Remove(ActorId),
    Move(ActorId, i8),
    ApplySeed,
    SaveFile,
    LoadFile,
}

#[derive(Debug, Clone)]
pub(super) struct ActorEditor {
    id: ActorId,
    revision: u64,
    draft: ScenarioActor,
    name: String,
    max_hp: String,
    speed: String,
    footprint: String,
    starting_hp: String,
    generation: u64,
    mounted: Option<u64>,
    error: Option<String>,
    pending_save: bool,
    submitted_revision: Option<u64>,
}
impl ActorEditor {
    fn new(draft: ScenarioActor, revision: u64) -> Self {
        Self {
            id: draft.id,
            name: draft.actor.name.clone(),
            max_hp: draft.actor.max_hp.to_string(),
            speed: draft.actor.base_speed.to_string(),
            footprint: draft.actor.footprint.to_string(),
            starting_hp: draft
                .starting_hp
                .map_or_else(String::new, |hp| hp.to_string()),
            draft,
            revision,
            generation: 0,
            mounted: None,
            error: None,
            pending_save: false,
            submitted_revision: None,
        }
    }
    fn validated(&self, catalog: &ContentCatalog) -> Result<ScenarioActor, String> {
        let mut draft = self.draft.clone();
        draft.actor.name = self.name.clone();
        draft.actor.max_hp = self
            .max_hp
            .parse()
            .map_err(|_| "Maximum HP must be a whole number.")?;
        draft.actor.base_speed = self
            .speed
            .parse()
            .map_err(|_| "Speed must be a whole number.")?;
        draft.actor.footprint = self
            .footprint
            .parse()
            .map_err(|_| "Footprint must be a whole number.")?;
        draft.starting_hp = if self.starting_hp.trim().is_empty() {
            None
        } else {
            Some(
                self.starting_hp
                    .parse()
                    .map_err(|_| "Starting HP must be blank or a whole number.")?,
            )
        };
        if draft.starting_hp.is_some_and(|hp| hp > draft.actor.max_hp) {
            return Err("Starting HP cannot exceed maximum HP.".into());
        }
        draft.actor.resolve(catalog).map_err(|e| e.to_string())?;
        Ok(draft)
    }
}

pub(super) fn change(ui: &mut UiState, field: BuildField, value: &str) {
    if let Some(editor) = &mut ui.editor {
        let current = match field {
            BuildField::Name => &mut editor.name,
            BuildField::MaxHp => &mut editor.max_hp,
            BuildField::Speed => &mut editor.speed,
            BuildField::Footprint => &mut editor.footprint,
            BuildField::StartingHp => &mut editor.starting_hp,
        };
        // EditableText layout/caret updates can emit the unchanged value.
        // Only an actual edit invalidates a submitted draft or its error.
        if current == value {
            return;
        }
        *current = value.into();
        editor.error = None;
        editor.pending_save = false;
        editor.submitted_revision = None;
    }
}

fn actor(scenario: &Scenario, id: ActorId) -> Option<&ScenarioActor> {
    scenario
        .heroes
        .iter()
        .chain(&scenario.enemies)
        .find(|a| a.id == id)
}

pub(super) fn action(
    view: &LabyrinthView,
    ui: &mut UiState,
    action: SetupAction,
) -> Option<LabyrinthIntent> {
    let scenario = view.scenario.as_ref()?;
    let catalog = view.catalog.as_ref()?;
    match action {
        SetupAction::Edit(id) => {
            ui.editor = actor(scenario, id)
                .cloned()
                .map(|a| ActorEditor::new(a, view.setup_revision));
        }
        SetupAction::Cancel => {
            ui.editor = None;
        }
        SetupAction::Reload => {
            if let Some(editor) = &ui.editor {
                ui.editor = actor(scenario, editor.id)
                    .cloned()
                    .map(|a| ActorEditor::new(a, view.setup_revision));
            }
        }
        SetupAction::Save => {
            let editor = ui.editor.as_mut()?;
            editor.error = None;
            editor.pending_save = false;
            if editor.revision != view.setup_revision {
                editor.error =
                    Some("Setup changed. Reload this character before applying your draft.".into());
                return None;
            }
            if !view.host
                && !view
                    .company
                    .iter()
                    .any(|m| m.actor == editor.id && Some(m.owner) == view.player)
            {
                editor.error = Some("This character is no longer assigned to you.".into());
                return None;
            }
            match editor.validated(catalog) {
                Ok(actor) => {
                    let mut candidate = scenario.clone();
                    if let Some(current) = candidate
                        .heroes
                        .iter_mut()
                        .chain(&mut candidate.enemies)
                        .find(|a| a.id == actor.id)
                    {
                        *current = actor.clone();
                    }
                    if let Err(error) = candidate.validate(catalog) {
                        editor.error = Some(error.to_string());
                        return None;
                    }
                    editor.pending_save = true;
                    editor.submitted_revision = Some(view.revision);
                    return Some(LabyrinthIntent::CustomizeActor {
                        actor,
                        expected_revision: editor.revision,
                    });
                }
                Err(error) => editor.error = Some(error),
            }
        }
        SetupAction::Preset(id) => {
            let editor = ui.editor.as_mut()?;
            match ActorBuild::from_preset(catalog, &id) {
                Ok(build) => {
                    let mut draft = editor.draft.clone();
                    draft.actor = build;
                    draft.starting_hp = None;
                    *editor = ActorEditor::new(draft, editor.revision);
                }
                Err(error) => editor.error = Some(error.to_string()),
            }
        }
        SetupAction::Weapon(id) => {
            let editor = ui.editor.as_mut()?;
            editor.draft.actor.build.weapon = id;
            editor.generation += 1;
            editor.error = None;
            editor.pending_save = false;
        }
        SetupAction::Innate(id) => {
            let editor = ui.editor.as_mut()?;
            let innate = &mut editor.draft.actor.build.innate;
            if innate.iter().any(|grant| grant.ability == id) {
                innate.retain(|grant| grant.ability != id);
            } else {
                innate.push(InnateGrant {
                    ability: id,
                    provenance: ContentId::new("innate").expect("constant ID"),
                });
            }
            editor.generation += 1;
            editor.error = None;
            editor.pending_save = false;
        }
        SetupAction::Learned(id) => {
            let editor = ui.editor.as_mut()?;
            let learned = &mut editor.draft.actor.build.learned_skills;
            if learned.contains(&id) {
                learned.retain(|skill| *skill != id);
            } else {
                learned.push(id);
            }
            editor.generation += 1;
            editor.error = None;
            editor.pending_save = false;
        }
        SetupAction::Status(kind) => {
            let editor = ui.editor.as_mut()?;
            if editor
                .draft
                .starting_statuses
                .iter()
                .any(|s| s.kind == kind)
            {
                editor.draft.starting_statuses.retain(|s| s.kind != kind);
            } else {
                editor
                    .draft
                    .starting_statuses
                    .push(labyrinth_rules::scenario::StartingStatus {
                        kind,
                        source: None,
                        remaining: None,
                    });
            }
            editor.generation += 1;
            editor.error = None;
            editor.pending_save = false;
        }
        SetupAction::Stock(index) => return Some(LabyrinthIntent::StockScenario(index)),
        SetupAction::SaveFile => {
            return Some(LabyrinthIntent::SaveScenario(ui.scenario_path.clone()))
        }
        SetupAction::LoadFile => {
            return Some(LabyrinthIntent::LoadScenario(ui.scenario_path.clone()))
        }
        SetupAction::ApplySeed => match ui.scenario_seed.parse::<u64>() {
            Ok(seed) => {
                let mut draft = scenario.clone();
                draft.seed = seed;
                return Some(LabyrinthIntent::ConfigureBattle {
                    scenario: draft,
                    expected_revision: view.setup_revision,
                });
            }
            Err(_) => {
                ui.local_notice =
                    Some("Seed must be a whole number from 0 to 18446744073709551615.".into())
            }
        },
        SetupAction::Remove(id) => {
            let mut draft = scenario.clone();
            draft.heroes.retain(|a| a.id != id);
            draft.enemies.retain(|a| a.id != id);
            return Some(LabyrinthIntent::ConfigureBattle {
                scenario: draft,
                expected_revision: view.setup_revision,
            });
        }
        SetupAction::Move(id, step) => {
            let mut draft = scenario.clone();
            let roster = if draft.heroes.iter().any(|a| a.id == id) {
                &mut draft.heroes
            } else {
                &mut draft.enemies
            };
            if let Some(index) = roster.iter().position(|a| a.id == id) {
                let next = index.saturating_add_signed(isize::from(step));
                if next < roster.len() {
                    roster.swap(index, next);
                }
            }
            return Some(LabyrinthIntent::ConfigureBattle {
                scenario: draft,
                expected_revision: view.setup_revision,
            });
        }
        SetupAction::Add(team) => return Some(LabyrinthIntent::AddScenarioActor(team)),
    }
    None
}

#[derive(Component)]
struct EditorRoot;
#[derive(Component)]
struct EditorNotice;
#[derive(Component)]
struct EditorSave;

pub(super) fn present(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    if view.mode != ViewMode::Lobby || !view.admitted {
        ui.editor = None;
    }
    if ui.editor.as_ref().is_some_and(|editor| {
        editor.pending_save
            && editor
                .submitted_revision
                .is_some_and(|revision| view.revision > revision)
            && view.setup_revision >= editor.revision
            && view
                .catalog
                .as_ref()
                .zip(view.scenario.as_ref())
                .is_some_and(|(catalog, scenario)| {
                    editor
                        .validated(catalog)
                        .is_ok_and(|draft| actor(scenario, editor.id) == Some(&draft))
                })
    }) {
        ui.editor = None;
    }
    let Some(editor) = &mut ui.editor else {
        despawn_marked::<EditorRoot>(world);
        return;
    };
    let Some(catalog) = &view.catalog else {
        return;
    };
    // An unrelated participant join must not remount editable text or lose its caret.
    let conflict = if view.setup_revision != editor.revision {
        "Setup changed. Reload this character before applying your draft."
    } else {
        ""
    };
    let notice = if !conflict.is_empty() {
        conflict.to_owned()
    } else {
        editor
            .error
            .as_deref()
            .or(view.notice.as_deref())
            .unwrap_or("")
            .to_owned()
    };
    for mut text in world
        .query_filtered::<&mut Text, With<EditorNotice>>()
        .iter_mut(world)
    {
        text.0.clone_from(&notice);
    }
    let can_apply = view.setup_revision == editor.revision
        && (view.host
            || view
                .company
                .iter()
                .any(|m| m.actor == editor.id && Some(m.owner) == view.player));
    let save_controls = world
        .query_filtered::<Entity, With<EditorSave>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in save_controls {
        set_disabled(world, entity, !can_apply);
    }
    if editor.mounted == Some(editor.generation) {
        return;
    }
    despawn_marked::<EditorRoot>(world);
    editor.mounted = Some(editor.generation);
    let root = world
        .spawn((
            bevy_gamekit::ui::menu_overlay("Character Editor"),
            EditorRoot,
        ))
        .id();
    let panel = world
        .spawn((
            bevy_gamekit::ui::menu_panel("Character Build"),
            ChildOf(root),
        ))
        .id();
    world.entity_mut(panel).insert(Node {
        width: Val::Percent(94.0),
        max_width: Val::Px(1050.0),
        max_height: Val::Percent(90.0),
        overflow: Overflow::scroll_y(),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.0),
        padding: UiRect::all(Val::Px(16.0)),
        ..default()
    });
    label(
        world,
        panel,
        "Build Title",
        format!("Character {} · edit draft", editor.id.0),
        UiTextRole::Title,
    );
    for (field_label, field, value, max) in [
        ("Name", BuildField::Name, &editor.name, 128),
        ("Maximum HP", BuildField::MaxHp, &editor.max_hp, 5),
        ("Speed", BuildField::Speed, &editor.speed, 5),
        (
            "Formation spaces",
            BuildField::Footprint,
            &editor.footprint,
            1,
        ),
        (
            "Starting HP (blank = full)",
            BuildField::StartingHp,
            &editor.starting_hp,
            5,
        ),
    ] {
        label(
            world,
            panel,
            &format!("Build {field:?} Label"),
            field_label,
            UiTextRole::Supporting,
        );
        let field_bundle =
            bevy_gamekit::ui::text_field(world.resource::<UiFonts>(), field_label, value, max);
        let entity = world
            .spawn((
                field_bundle,
                UiSkin::Field,
                Field::Build(field),
                bevy_gamekit::ui::UiFocusId::new(
                    "labyrinth-build",
                    format!("{}:{field:?}", editor.id.0),
                ),
                ChildOf(panel),
            ))
            .id();
        if matches!(field, BuildField::Footprint) && !view.host {
            world
                .entity_mut(entity)
                .insert(bevy_gamekit::ui::UiDisabled);
        }
    }
    let presets = shell::row(world, panel, "Build Presets");
    for preset in &catalog.definition().actor_presets {
        control(
            world,
            presets,
            format!("Build Preset {}", preset.id),
            &preset.name,
            Action::Setup(SetupAction::Preset(preset.id.clone())),
            false,
        );
    }
    label(
        world,
        panel,
        "Weapon Title",
        "Weapon · one optional item",
        UiTextRole::Body,
    );
    let weapons = shell::row(world, panel, "Build Weapons");
    control(
        world,
        weapons,
        "Weapon None",
        "Unarmed",
        Action::Setup(SetupAction::Weapon(None)),
        editor.draft.actor.build.weapon.is_none(),
    );
    for weapon in &catalog.definition().weapons {
        control(
            world,
            weapons,
            format!("Weapon {}", weapon.id),
            &weapon.name,
            Action::Setup(SetupAction::Weapon(Some(weapon.id.clone()))),
            editor.draft.actor.build.weapon.as_ref() == Some(&weapon.id),
        );
    }
    label(
        world,
        panel,
        "Innate Title",
        "Innate active abilities",
        UiTextRole::Body,
    );
    let innate = shell::row(world, panel, "Build Innate");
    for ability in &catalog.definition().abilities {
        let selected = editor
            .draft
            .actor
            .build
            .innate
            .iter()
            .any(|g| g.ability == ability.id);
        control(
            world,
            innate,
            format!("Innate {}", ability.id),
            format!("{}{}", if selected { "✓ " } else { "" }, ability.name),
            Action::Setup(SetupAction::Innate(ability.id.clone())),
            false,
        );
    }
    label(
        world,
        panel,
        "Learned Title",
        "Learned skills · additions and upgrades",
        UiTextRole::Body,
    );
    let learned = shell::row(world, panel, "Build Learned");
    for skill in &catalog.definition().learned_skills {
        control(
            world,
            learned,
            format!("Learned {}", skill.id),
            format!(
                "{}{}",
                if editor.draft.actor.build.learned_skills.contains(&skill.id) {
                    "✓ "
                } else {
                    ""
                },
                skill.name
            ),
            Action::Setup(SetupAction::Learned(skill.id.clone())),
            false,
        );
    }
    label(
        world,
        panel,
        "Starting Status Title",
        "Starting conditions",
        UiTextRole::Body,
    );
    let statuses = shell::row(world, panel, "Starting Statuses");
    for kind in [
        labyrinth_rules::StatusKind::Bleed,
        labyrinth_rules::StatusKind::Brace,
        labyrinth_rules::StatusKind::Haste,
        labyrinth_rules::StatusKind::Weakened,
    ] {
        control(
            world,
            statuses,
            format!("Starting {kind:?}"),
            format!(
                "{}{kind:?}",
                if editor
                    .draft
                    .starting_statuses
                    .iter()
                    .any(|s| s.kind == kind)
                {
                    "✓ "
                } else {
                    ""
                }
            ),
            Action::Setup(SetupAction::Status(kind)),
            false,
        );
    }
    let summary = editor
        .validated(catalog)
        .and_then(|a| a.actor.resolve(catalog).map_err(|e| e.to_string()))
        .map_or_else(
            |error| error,
            |build| {
                format!(
                    "{} available active abilities: {}",
                    build.abilities.len(),
                    build
                        .abilities
                        .iter()
                        .map(|a| a.definition.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        );
    label(
        world,
        panel,
        "Resolved Build",
        summary,
        UiTextRole::Supporting,
    );
    let notice_entity = label(world, panel, "Build Notice", notice, UiTextRole::Body);
    world.entity_mut(notice_entity).insert(EditorNotice);
    let actions = shell::row(world, panel, "Build Actions");
    let save_control = control(
        world,
        actions,
        "Apply Build",
        "Apply build",
        Action::Setup(SetupAction::Save),
        !can_apply,
    );
    world.entity_mut(save_control).insert(EditorSave);
    control(
        world,
        actions,
        "Reload Build",
        "Reload current character",
        Action::Setup(SetupAction::Reload),
        false,
    );
    control(
        world,
        actions,
        "Close Build",
        "Close editor",
        Action::Setup(SetupAction::Cancel),
        false,
    );
}

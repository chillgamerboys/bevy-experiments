//! Local actor drafts. Authority validates every submitted scenario/build again.

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "setup/tests.rs"]
mod decision_tests;
pub(super) mod details;
mod layout;

use super::*;
use labyrinth_rules::build::{ActorBuild, SkillGrant};
use labyrinth_rules::catalog::{ContentCatalog, ContentId};
use labyrinth_rules::scenario::{Scenario, ScenarioActor};
use std::collections::BTreeMap;

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
    Category(Category),
    Inspect(ActorId, Selection),
    Browse,
    ApplyInspected(ActorId, Selection),
    ConfirmDiscard,
    KeepEditing,
    Preset(ContentId),
    Weapon(Option<ContentId>),
    Skill(ContentId),
    Ability(ContentId),
    Status(labyrinth_rules::StatusKind),
    Save,
    Cancel,
    Close,
    Reload,
    Stock(usize),
    ApplySeed,
    SaveFile,
    LoadFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Category {
    Equipment,
    Skills,
    Abilities,
    Parameters,
    Moveset,
}
impl Category {
    const ALL: [Self; 5] = [
        Self::Equipment,
        Self::Skills,
        Self::Abilities,
        Self::Parameters,
        Self::Moveset,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Equipment => "Equipment",
            Self::Skills => "Skills",
            Self::Abilities => "Abilities",
            Self::Parameters => "Parameters",
            Self::Moveset => "Moveset",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Selection {
    Weapon(Option<ContentId>),
    Skill(ContentId),
    Ability(ContentId),
    Preset(ContentId),
    Move(ContentId),
}
#[derive(Debug, Clone)]
enum ExitTarget {
    Close,
    Actor(ActorId),
}

#[derive(Debug, Clone)]
pub(super) struct ActorEditor {
    id: ActorId,
    original: ScenarioActor,
    category: Category,
    selections: BTreeMap<Category, Selection>,
    detail_only: bool,
    scrolls: BTreeMap<Category, f32>,
    pending_exit: Option<ExitTarget>,
    compact: bool,
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
            original: draft.clone(),
            category: Category::Equipment,
            selections: BTreeMap::new(),
            detail_only: false,
            scrolls: BTreeMap::new(),
            pending_exit: None,
            compact: false,
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
    fn dirty(&self) -> bool {
        self.draft != self.original
            || self.name != self.original.actor.name
            || self.max_hp != self.original.actor.max_hp.to_string()
            || self.speed != self.original.actor.base_speed.to_string()
            || self.footprint != self.original.actor.footprint.to_string()
            || self.starting_hp
                != self
                    .original
                    .starting_hp
                    .map_or_else(String::new, |hp| hp.to_string())
    }
    fn selection(&self) -> Option<&Selection> {
        self.selections.get(&self.category)
    }
    fn ensure_selection(&mut self, catalog: &ContentCatalog) {
        if self.selections.contains_key(&self.category) {
            return;
        }
        let selection = match self.category {
            Category::Equipment => Some(Selection::Weapon(
                self.draft.actor.build.weapon.clone().or_else(|| {
                    catalog
                        .definition()
                        .weapons
                        .first()
                        .map(|weapon| weapon.id.clone())
                }),
            )),
            Category::Skills => catalog
                .definition()
                .skills
                .first()
                .map(|a| Selection::Skill(a.id.clone())),
            Category::Abilities => catalog
                .definition()
                .abilities
                .first()
                .map(|a| Selection::Ability(a.id.clone())),
            Category::Parameters => None,
            Category::Moveset => catalog
                .resolve_build(&self.draft.actor.build)
                .ok()
                .and_then(|build| {
                    build
                        .moveset
                        .skills
                        .first()
                        .map(|a| Selection::Move(a.definition.id.clone()))
                }),
        };
        if let Some(selection) = selection {
            self.selections.insert(self.category, selection);
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

pub(super) fn change_from_field(ui: &mut UiState, field: BuildField, value: &str) {
    // A queued event from the prior mounted draft must not overwrite Reload,
    // preset selection, or another character before the replacement is mounted.
    if ui
        .editor
        .as_ref()
        .is_some_and(|editor| editor.mounted == Some(editor.generation))
    {
        change(ui, field, value);
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
    requested: SetupAction,
) -> Option<LabyrinthIntent> {
    let scenario = view.scenario.as_ref()?;
    let catalog = view.catalog.as_ref()?;
    match requested {
        SetupAction::Edit(id) => {
            if let Some(editor) = &mut ui.editor {
                if editor.id == id {
                    return None;
                }
                if editor.dirty() {
                    editor.pending_exit = Some(ExitTarget::Actor(id));
                    editor.generation += 1;
                    return None;
                }
            }
            ui.editor = actor(scenario, id)
                .cloned()
                .map(|a| ActorEditor::new(a, view.setup_revision));
        }
        SetupAction::Cancel => {
            if let Some(editor) = &mut ui.editor {
                if editor.pending_exit.is_some() {
                    editor.pending_exit = None;
                    editor.generation += 1;
                } else if editor.compact && editor.detail_only {
                    editor.detail_only = false;
                    editor.generation += 1;
                } else if editor.dirty() {
                    editor.pending_exit = Some(ExitTarget::Close);
                    editor.generation += 1;
                } else {
                    ui.editor = None;
                }
            }
        }
        SetupAction::Close => {
            if let Some(editor) = &mut ui.editor {
                if editor.dirty() {
                    editor.pending_exit = Some(ExitTarget::Close);
                    editor.generation += 1;
                } else {
                    ui.editor = None;
                }
            }
        }
        SetupAction::ConfirmDiscard => {
            let target = ui.editor.as_mut()?.pending_exit.take()?;
            ui.editor = match target {
                ExitTarget::Close => None,
                ExitTarget::Actor(id) => actor(scenario, id)
                    .cloned()
                    .map(|a| ActorEditor::new(a, view.setup_revision)),
            };
        }
        SetupAction::KeepEditing => {
            let editor = ui.editor.as_mut()?;
            editor.pending_exit = None;
            editor.generation += 1;
        }
        SetupAction::Category(category) => {
            let editor = ui.editor.as_mut()?;
            editor.category = category;
            editor.detail_only = false;
            editor.ensure_selection(catalog);
            editor.generation += 1;
        }
        SetupAction::Inspect(id, selection) => {
            let editor = ui.editor.as_mut()?;
            if editor.id != id {
                return None;
            }
            let category = match selection {
                Selection::Weapon(_) => Category::Equipment,
                Selection::Skill(_) => Category::Skills,
                Selection::Ability(_) => Category::Abilities,
                Selection::Preset(_) => Category::Parameters,
                Selection::Move(_) => Category::Moveset,
            };
            if editor.category != category {
                return None;
            }
            editor.selections.insert(editor.category, selection);
            editor.detail_only = true;
            editor.generation += 1;
        }
        SetupAction::Browse => {
            let editor = ui.editor.as_mut()?;
            editor.detail_only = false;
            editor.generation += 1;
        }
        SetupAction::ApplyInspected(id, selected) => {
            let editor = ui.editor.as_ref()?;
            if editor.id != id
                || editor.selection() != Some(&selected)
                || !layout::editable(view, id)
            {
                return None;
            }
            if details::inspection(editor, catalog)
                .apply
                .is_none_or(|(_, disabled)| disabled)
            {
                return None;
            }
            let edit = match selected {
                Selection::Weapon(id) => SetupAction::Weapon(id),
                Selection::Skill(id) => SetupAction::Skill(id),
                Selection::Ability(id) => SetupAction::Ability(id),
                Selection::Preset(id) => SetupAction::Preset(id),
                Selection::Move(_) => return None,
            };
            return action(view, ui, edit);
        }
        SetupAction::Reload => {
            if let Some(editor) = &ui.editor {
                let category = editor.category;
                let selections = editor.selections.clone();
                let scrolls = editor.scrolls.clone();
                ui.editor = actor(scenario, editor.id).cloned().map(|a| {
                    let mut refreshed = ActorEditor::new(a, view.setup_revision);
                    refreshed.category = category;
                    refreshed.selections = selections;
                    refreshed.scrolls = scrolls;
                    refreshed
                });
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
                    if let Some(formation) = &view.formation {
                        let team = if scenario.heroes.iter().any(|a| a.id == actor.id) {
                            labyrinth_rules::Team::Heroes
                        } else {
                            labyrinth_rules::Team::Enemies
                        };
                        if let Some(rank) = formation.rank(actor.id) {
                            if let Some(error) = formation.placement_error(
                                scenario,
                                team,
                                rank,
                                actor.actor.footprint,
                                Some(actor.id),
                                if view.host { None } else { view.player },
                            ) {
                                editor.error = Some(error);
                                return None;
                            }
                        }
                    }
                    if let Err(error) = candidate.validate_preparation(catalog) {
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
                    let original = editor.original.clone();
                    let category = editor.category;
                    let selections = editor.selections.clone();
                    let next_generation = editor.generation + 1;
                    *editor = ActorEditor::new(draft, editor.revision);
                    editor.original = original;
                    editor.category = category;
                    editor.selections = selections;
                    editor.generation = next_generation;
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
        SetupAction::Skill(id) => {
            if catalog.skill(&id).is_none_or(|s| !s.personal_selectable) {
                return None;
            }
            let editor = ui.editor.as_mut()?;
            let skills = &mut editor.draft.actor.build.skills;
            if skills.iter().any(|grant| grant.skill == id) {
                skills.retain(|grant| grant.skill != id);
            } else {
                skills.push(SkillGrant {
                    skill: id,
                    provenance: ContentId::new("skills").expect("constant ID"),
                });
            }
            editor.generation += 1;
            editor.error = None;
            editor.pending_save = false;
        }
        SetupAction::Ability(id) => {
            if catalog.ability(&id).is_none_or(|a| !a.personal_selectable) {
                return None;
            }
            let editor = ui.editor.as_mut()?;
            let learned = &mut editor.draft.actor.build.abilities;
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
        SetupAction::Stock(index) => {
            ui.local_notice = None;
            return Some(LabyrinthIntent::StockScenario(index));
        }
        SetupAction::SaveFile => {
            ui.local_notice = None;
            return Some(LabyrinthIntent::SaveScenario(ui.scenario_path.clone()));
        }
        SetupAction::LoadFile => {
            ui.local_notice = None;
            return Some(LabyrinthIntent::LoadScenario(ui.scenario_path.clone()));
        }
        SetupAction::ApplySeed => match ui.scenario_seed.parse::<u64>() {
            Ok(seed) => {
                ui.local_notice = None;
                return Some(LabyrinthIntent::SetScenarioSeed {
                    seed,
                    expected_revision: view.setup_revision,
                });
            }
            Err(_) => {
                ui.local_notice =
                    Some("Seed must be a whole number from 0 to 18446744073709551615.".into())
            }
        },
    }
    None
}

pub(super) fn present(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    layout::present(world, view, ui);
}

pub(super) fn scroll_details(world: &mut World, direction: i8) {
    layout::scroll_details(world, direction);
}

//! Spatial preparation selection is local; every mutation is a revision-bound intent.
mod layout;
use super::*;
use crate::view::{FormationPlacement, LobbyFormation};
use labyrinth_rules::{catalog::ContentId, scenario::ScenarioActor, Team};

#[derive(Debug, Clone, Default)]
pub(super) struct ConstructorState {
    pub selection: Option<(Team, u8)>,
    mode: ConstructorMode,
    preset: Option<ContentId>,
    revision: u64,
    owners: bool,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ConstructorMode {
    #[default]
    Inspect,
    Pick,
    Move(ActorId),
}
#[derive(Debug, Clone)]
pub(super) enum ConstructorAction {
    Select(Team, u8),
    Pick,
    InspectType(ContentId),
    BrowseTypes,
    Move(ActorId),
    Commit(PlacementProposal),
    Remove(ActorId, u64),
    Owners,
    Assign { rank: u8, owner: u8, revision: u64 },
    Close,
}

/// The exact candidate the mounted confirmation describes; not just an authority revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlacementProposal {
    team: Team,
    rank: u8,
    revision: u64,
    mode: ConstructorMode,
    preset: Option<ContentId>,
    replaced: Option<ActorId>,
}
fn proposal(view: &LabyrinthView, state: &ConstructorState) -> Option<PlacementProposal> {
    let (team, rank) = state.selection?;
    if state.mode == ConstructorMode::Inspect
        || (state.mode == ConstructorMode::Pick && state.preset.is_none())
    {
        return None;
    }
    Some(PlacementProposal {
        team,
        rank,
        revision: state.revision,
        mode: state.mode,
        preset: state.preset.clone(),
        replaced: selected(view, state),
    })
}

pub(super) fn compact(metrics: ResolvedUiMetrics) -> bool {
    metrics.logical_size.x / metrics.content_scale < 1000.0
}

pub(super) fn formation(view: &LabyrinthView) -> LobbyFormation {
    if let Some(formation) = &view.formation {
        return formation.clone();
    }
    let mut formation = LobbyFormation {
        heroes: Vec::new(),
        enemies: Vec::new(),
        hero_owners: [0; 6],
    };
    if let Some(scenario) = &view.scenario {
        for (roster, placements) in [
            (&scenario.heroes, &mut formation.heroes),
            (&scenario.enemies, &mut formation.enemies),
        ] {
            let mut rank = 1;
            for actor in roster {
                placements.push(FormationPlacement {
                    actor: actor.id,
                    rank,
                });
                rank += actor.actor.footprint;
            }
        }
        for place in &formation.heroes {
            if let Some(actor) = scenario.heroes.iter().find(|a| a.id == place.actor) {
                let owner = view
                    .company
                    .iter()
                    .find(|m| m.actor == actor.id)
                    .map_or(0, |m| m.owner);
                for rank in place.rank..place.rank + actor.actor.footprint {
                    if let Some(value) = formation.hero_owners.get_mut(usize::from(rank - 1)) {
                        *value = owner;
                    }
                }
            }
        }
    }
    formation
}
fn actor(view: &LabyrinthView, id: ActorId) -> Option<&ScenarioActor> {
    view.scenario
        .as_ref()?
        .heroes
        .iter()
        .chain(&view.scenario.as_ref()?.enemies)
        .find(|a| a.id == id)
}
fn selected(view: &LabyrinthView, state: &ConstructorState) -> Option<ActorId> {
    let (team, rank) = state.selection?;
    formation(view).occupant(view.scenario.as_ref()?, team, rank)
}
fn can_choose(view: &LabyrinthView, state: &ConstructorState) -> bool {
    let Some((team, rank)) = state.selection else {
        return false;
    };
    view.admitted
        && (view.host
            || (team == Team::Heroes
                && view.player == Some(formation(view).owner(rank).unwrap_or(0))))
}
fn preview_error(view: &LabyrinthView, state: &ConstructorState) -> Option<String> {
    if !view.admitted {
        return Some(
            "Reconnect to choose or change characters. Inspection stays available.".into(),
        );
    }
    if state.revision != view.setup_revision {
        return Some(
            "The formation changed. Select the place again to review the new lineup.".into(),
        );
    }
    let (team, rank) = state.selection?;
    let scenario = view.scenario.as_ref()?;
    let (footprint, replacing) = match state.mode {
        ConstructorMode::Pick => {
            let preset = view
                .catalog
                .as_ref()?
                .actor_preset(state.preset.as_ref()?)?;
            (preset.footprint, selected(view, state))
        }
        ConstructorMode::Move(id) => (actor(view, id)?.actor.footprint, Some(id)),
        ConstructorMode::Inspect => return None,
    };
    formation(view).placement_error(
        scenario,
        team,
        rank,
        footprint,
        replacing,
        if view.host { None } else { view.player },
    )
}
pub(super) fn action(
    view: &LabyrinthView,
    ui: &mut UiState,
    requested: ConstructorAction,
) -> Option<LabyrinthIntent> {
    if view.mode != ViewMode::Lobby {
        return None;
    }
    let state = &mut ui.constructor;
    match requested {
        ConstructorAction::Select(team, mut rank) => {
            if !(1..=6).contains(&rank) {
                return None;
            }
            let formation = formation(view);
            let occupied = formation.occupant(view.scenario.as_ref()?, team, rank);
            // A movement destination is the exact rank chosen. Inspection selects a whole creature.
            if !matches!(state.mode, ConstructorMode::Move(_)) {
                if let Some(id) = occupied {
                    rank = formation.rank(id)?;
                }
                state.mode = if occupied.is_some() {
                    ConstructorMode::Inspect
                } else {
                    ConstructorMode::Pick
                };
                state.preset = None;
            } else if state
                .selection
                .is_some_and(|(selected_team, _)| selected_team != team)
            {
                return None;
            }
            state.selection = Some((team, rank));
            state.revision = view.setup_revision;
            state.owners = false;
            ui.lobby_page = 0;
        }
        ConstructorAction::Pick => {
            if !can_choose(view, state) {
                return None;
            }
            state.mode = ConstructorMode::Pick;
            state.preset = None;
            state.revision = view.setup_revision;
        }
        ConstructorAction::InspectType(id) => {
            let preset = view.catalog.as_ref()?.actor_preset(&id)?;
            let (team, _) = state.selection?;
            if preset.appearance.team() != team || state.mode != ConstructorMode::Pick {
                return None;
            }
            state.preset = Some(id);
        }
        ConstructorAction::BrowseTypes => state.preset = None,
        ConstructorAction::Move(id) => {
            if !view.admitted || !view.host || selected(view, state) != Some(id) {
                return None;
            }
            state.mode = ConstructorMode::Move(id);
            state.preset = None;
            state.revision = view.setup_revision;
        }
        ConstructorAction::Commit(bound) => {
            if proposal(view, state).as_ref() != Some(&bound)
                || bound.revision != view.setup_revision
                || preview_error(view, state).is_some()
            {
                return None;
            }
            let PlacementProposal {
                team,
                rank,
                revision,
                mode,
                preset,
                ..
            } = bound;
            let intent = match mode {
                ConstructorMode::Pick if can_choose(view, state) => {
                    LabyrinthIntent::PlaceScenarioActor {
                        team,
                        rank,
                        preset: preset?,
                        expected_revision: revision,
                    }
                }
                ConstructorMode::Move(id) if view.host => LabyrinthIntent::MoveScenarioActor {
                    actor: id,
                    rank,
                    expected_revision: revision,
                },
                _ => return None,
            };
            state.mode = ConstructorMode::Inspect;
            state.preset = None;
            return Some(intent);
        }
        ConstructorAction::Remove(id, revision) => {
            if !view.admitted
                || !view.host
                || revision != view.setup_revision
                || selected(view, state) != Some(id)
            {
                return None;
            }
            state.mode = ConstructorMode::Pick;
            state.preset = None;
            return Some(LabyrinthIntent::RemoveScenarioActor {
                actor: id,
                expected_revision: revision,
            });
        }
        ConstructorAction::Owners => state.owners = !state.owners,
        ConstructorAction::Assign {
            rank,
            owner,
            revision,
        } => {
            if !view.admitted
                || !view.host
                || state.selection != Some((Team::Heroes, rank))
                || revision != view.setup_revision
            {
                return None;
            }
            state.owners = false;
            return Some(LabyrinthIntent::AssignFormationRank {
                rank,
                owner,
                expected_revision: revision,
            });
        }
        ConstructorAction::Close => {
            if let ConstructorMode::Move(id) = state.mode {
                if let Some((team, _)) = state.selection {
                    state.selection = formation(view).rank(id).map(|rank| (team, rank));
                }
                state.mode = ConstructorMode::Inspect;
                state.preset = None;
            } else if state.mode == ConstructorMode::Pick && selected(view, state).is_some() {
                state.mode = ConstructorMode::Inspect;
                state.preset = None;
            } else {
                *state = ConstructorState::default();
            }
        }
    }
    None
}
pub(super) fn refresh(view: &LabyrinthView, ui: &mut UiState) {
    if view.mode != ViewMode::Lobby {
        ui.constructor = ConstructorState::default();
    }
    if ui.constructor.mode == ConstructorMode::Pick && ui.constructor.preset.is_none() {
        ui.constructor.revision = view.setup_revision;
    }
}
pub(super) fn board(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    layout::board(world, parent, view, &ui.constructor);
}
pub(super) fn context(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    layout::context(world, parent, view, &ui.constructor);
}
pub(super) fn commit(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    let state = &ui.constructor;
    let Some((_, rank)) = state.selection else {
        return;
    };
    let text = match state.mode {
        ConstructorMode::Pick if state.preset.is_some() => format!(
            "{} {} · Rank {rank}",
            if selected(view, state).is_some() {
                "Replace with"
            } else {
                "Place"
            },
            state
                .preset
                .as_ref()
                .and_then(|id| view.catalog.as_ref()?.actor_preset(id))
                .map_or("character", |p| p.name.as_str())
        ),
        ConstructorMode::Move(_) => format!("Move to rank {rank}"),
        _ => return,
    };
    let disabled = preview_error(view, state).is_some() || !can_choose(view, state);
    let button = control(
        world,
        parent,
        "Commit Placement",
        text,
        Action::Constructor(ConstructorAction::Commit(
            proposal(view, state).expect("visible placement proposal"),
        )),
        disabled,
    );
    world.entity_mut(button).insert(UiSkinOverrides {
        background: Some(Color::srgb(0.28, 0.24, 0.13)),
        disabled: Some(Color::srgb(0.07, 0.08, 0.08)),
        border: Some(if disabled {
            Color::srgb(0.23, 0.26, 0.25)
        } else {
            Color::srgb(0.89, 0.75, 0.43)
        }),
        ..default()
    });
    if disabled {
        let children = world
            .get::<Children>(button)
            .map(|c| c.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in children {
            world.entity_mut(child).insert(UiSkinOverrides {
                text: Some(Color::srgb(0.45, 0.48, 0.46)),
                ..default()
            });
        }
    }
}

pub(super) fn deployment_summary(view: &LabyrinthView) -> Option<String> {
    let scenario = view.scenario.as_ref()?;
    let formation = formation(view);
    for (team, name) in [(Team::Heroes, "party"), (Team::Enemies, "enemy")] {
        let last = formation
            .placements(team)
            .iter()
            .filter_map(|p| actor(view, p.actor).map(|a| p.rank + a.actor.footprint - 1))
            .max()
            .unwrap_or(0);
        if let Some(rank) =
            (1..=last).find(|rank| formation.occupant(scenario, team, *rank).is_none())
        {
            return Some(format!("Fill {name} rank {rank} to deploy"));
        }
    }
    None
}

pub(super) fn scroll_details(world: &mut World, direction: i8, constructor: bool) {
    let target = if constructor {
        "Construction Detail Scroll"
    } else {
        "Preparation Content"
    };
    let Some((entity, node, position)) = world
        .query::<(Entity, &Name, &ComputedNode, &ScrollPosition)>()
        .iter(world)
        .find_map(|(e, name, node, position)| {
            (name.as_str() == target).then_some((e, node, position))
        })
    else {
        return;
    };
    let height = node.size().y * node.inverse_scale_factor;
    let max = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
    let next = match direction {
        100.. => max,
        ..=-100 => 0.0,
        _ => (position.y + f32::from(direction) * height * 0.85).clamp(0.0, max),
    };
    world
        .entity_mut(entity)
        .insert(ScrollPosition(Vec2::new(0.0, next)));
}

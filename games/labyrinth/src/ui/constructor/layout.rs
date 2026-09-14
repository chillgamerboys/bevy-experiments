//! Facing formation art, contextual selection and inspect-before-placement choices.
use super::*;
use labyrinth_rules::{ActorKind, EnemyKind, HeroClass};

fn row(world: &mut World, parent: Entity, name: &str) -> Entity {
    shell::row(world, parent, name)
}
fn owner_color(owner: u8) -> Color {
    const COLORS: [[f32; 3]; 6] = [
        [0.91, 0.76, 0.45],
        [0.47, 0.79, 0.74],
        [0.71, 0.63, 0.91],
        [0.91, 0.57, 0.44],
        [0.56, 0.74, 0.94],
        [0.79, 0.84, 0.50],
    ];
    Color::srgb_from_array(
        *COLORS
            .get(usize::from(owner))
            .unwrap_or(&[0.91, 0.76, 0.45]),
    )
}
fn quiet(world: &mut World, entity: Entity, selected: bool) {
    world.entity_mut(entity).insert(UiSkinOverrides {
        background: Some(if selected {
            Color::srgba(0.34, 0.28, 0.12, 0.30)
        } else {
            Color::NONE
        }),
        hovered: Some(Color::srgba(0.40, 0.44, 0.39, 0.18)),
        pressed: Some(Color::srgba(0.52, 0.45, 0.25, 0.25)),
        border: Some(if selected {
            Color::srgb(0.89, 0.75, 0.43)
        } else {
            Color::srgba(0.60, 0.64, 0.57, 0.24)
        }),
        ..default()
    });
}
fn text_color(world: &mut World, entity: Entity, color: Color) {
    world.entity_mut(entity).insert(UiSkinOverrides {
        text: Some(color),
        ..default()
    });
}
fn art(
    world: &mut World,
    parent: Entity,
    name: String,
    kind: ActorKind,
    area: Vec2,
    preview: bool,
    floor: f32,
) {
    let appearance = world.get_resource::<crate::scene::SceneAppearance>();
    let image = appearance.and_then(|a| {
        let handle = a.actor_image(kind)?.clone();
        let image = world.get_resource::<Assets<Image>>()?.get(&handle)?;
        Some(ImageNode {
            rect: a.actor_rect(image.size(), kind),
            image: handle,
            color: Color::srgba(1.0, 1.0, 1.0, if preview { 0.56 } else { 1.0 }),
            ..default()
        })
    });
    let Some(image) = image else {
        return;
    };
    let size = crate::scene::actor_art_size(world, kind, area).unwrap_or(area);
    world.spawn((
        Name::new(name),
        image,
        Node {
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            flex_shrink: 0.0,
            margin: UiRect::bottom(Val::Px(floor)),
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(parent),
    ));
}
fn role(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => "Frontline defender",
        ActorKind::Hero(HeroClass::Knifehand) => "Close-range striker",
        ActorKind::Hero(HeroClass::Scout) => "Ranged attacker",
        ActorKind::Hero(HeroClass::FieldMedic) => "Healer and support",
        ActorKind::Hero(HeroClass::LanternWagon) => "Two-rank support",
        ActorKind::Enemy(EnemyKind::AshBrute | EnemyKind::IronBrute) => "Frontline attacker",
        ActorKind::Enemy(EnemyKind::WoundStalker) => "Bleed attacker",
        ActorKind::Enemy(EnemyKind::HollowArcher) => "Ranged attacker",
        ActorKind::Enemy(EnemyKind::OssuaryHauler) => "Two-rank attacker",
    }
}
fn team_name(team: Team) -> &'static str {
    if team == Team::Heroes {
        "Party"
    } else {
        "Enemies"
    }
}
fn span(rank: u8, footprint: u8) -> String {
    if footprint == 1 {
        format!("Rank {rank}")
    } else {
        format!("Ranks {rank}–{}", rank.saturating_add(footprint - 1))
    }
}

pub(super) fn board(
    world: &mut World,
    parent: Entity,
    view: &LabyrinthView,
    state: &ConstructorState,
) {
    let Some(scenario) = &view.scenario else {
        return;
    };
    let formation = formation(view);
    let metrics = *world.resource::<ResolvedUiMetrics>();
    let scale = metrics.content_scale;
    let mini = compact(metrics) && state.selection.is_some();
    let height = if mini {
        132.0
    } else if compact(metrics) {
        250.0
    } else {
        (metrics.logical_size.y * 0.27).clamp(190.0, 320.0)
    };
    let stage = column(
        world,
        parent,
        "Construction Battlefield",
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(height + if mini { 44.0 } else { 36.0 * scale }),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(20.0),
            ..default()
        },
    );
    let front = label(
        world,
        stage,
        "Construction Front",
        "FRONT",
        UiTextRole::Supporting,
    );
    world.entity_mut(front).insert(Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(47.0),
        top: Val::Px(0.0),
        ..default()
    });
    for team in [Team::Heroes, Team::Enemies] {
        let half = column(
            world,
            stage,
            &format!("{team:?} Construction"),
            Node {
                width: Val::Percent(50.0),
                min_width: Val::Px(0.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        );
        let title = label(
            world,
            half,
            &format!("{team:?} Setup Title"),
            if team == Team::Heroes {
                "PARTY"
            } else {
                "ENEMIES"
            },
            UiTextRole::Supporting,
        );
        world.entity_mut(title).insert((
            Node {
                width: Val::Percent(100.0),
                margin: UiRect::bottom(Val::Px(6.0)),
                ..default()
            },
            TextLayout::justify(if team == Team::Heroes {
                Justify::Left
            } else {
                Justify::Right
            }),
        ));
        let ground = column(
            world,
            half,
            &format!("{team:?} Rank Anchors"),
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                position_type: PositionType::Relative,
                ..default()
            },
        );
        let used = formation
            .placements(team)
            .iter()
            .filter_map(|p| actor(view, p.actor).map(|a| p.rank + a.actor.footprint - 1))
            .max()
            .unwrap_or(0);
        for rank in 1..=6 {
            let occupant = formation.occupant(scenario, team, rank);
            let selected_rank = state.selection.is_some_and(|(t, r)| {
                t == team
                    && (r == rank || occupant.is_some_and(|id| Some(id) == selected(view, state)))
            });
            let blocked_gap = occupant.is_none() && rank <= used;
            let left = if team == Team::Heroes {
                6 - rank
            } else {
                rank - 1
            };
            let anchor = column(
                world,
                ground,
                &format!("{team:?} Rank {rank} Ground"),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(f32::from(left) * 100.0 / 6.0),
                    width: Val::Percent(100.0 / 6.0),
                    height: Val::Px(if mini { 50.0 } else { 46.0 * scale }),
                    bottom: Val::Px(0.0),
                    border: UiRect::top(Val::Px(if selected_rank { 3.0 } else { 1.0 })),
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            );
            world.entity_mut(anchor).insert((
                BorderColor::all(if selected_rank {
                    Color::srgb(0.89, 0.75, 0.43)
                } else if blocked_gap {
                    Color::srgb(0.95, 0.48, 0.37)
                } else {
                    Color::srgba(0.66, 0.69, 0.59, 0.35)
                }),
                BackgroundColor(if blocked_gap {
                    Color::srgba(0.38, 0.10, 0.06, 0.34)
                } else {
                    Color::srgba(0.05, 0.07, 0.07, 0.56)
                }),
            ));
            let number = label(
                world,
                anchor,
                &format!("{team:?} Rank {rank} Label"),
                if blocked_gap {
                    format!("{rank} !")
                } else {
                    rank.to_string()
                },
                UiTextRole::Body,
            );
            if blocked_gap {
                text_color(world, number, Color::srgb(1.0, 0.63, 0.48));
            }
            if occupant.is_none() || matches!(state.mode, ConstructorMode::Move(_)) {
                let button = control(
                    world,
                    ground,
                    format!("Select {team:?} Rank {rank}"),
                    if occupant.is_none() { "+" } else { "" },
                    Action::Constructor(ConstructorAction::Select(team, rank)),
                    false,
                );
                world.entity_mut(button).remove::<UiSpacing>().insert(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(f32::from(left) * 100.0 / 6.0),
                    bottom: Val::Px(if mini { 50.0 } else { 50.0 * scale }),
                    width: Val::Percent(100.0 / 6.0),
                    height: Val::Px(if matches!(state.mode, ConstructorMode::Move(_)) {
                        height - 50.0
                    } else {
                        80.0
                    }),
                    border: UiRect::bottom(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                });
                quiet(world, button, selected_rank);
                world
                    .get_mut::<UiSkinOverrides>(button)
                    .expect("rank skin")
                    .background = Some(Color::NONE);
                let accessible = if matches!(state.mode, ConstructorMode::Move(_)) {
                    let occupancy = occupant.and_then(|id| actor(view, id)).map_or_else(
                        || "empty".to_owned(),
                        |a| format!("occupied by {}", a.actor.name),
                    );
                    format!(
                        "{} rank {rank}, {occupancy}{}. Choose this movement destination.",
                        team_name(team),
                        if blocked_gap { ", deployment gap" } else { "" }
                    )
                } else {
                    format!(
                        "{} rank {rank}, empty{}. Select a character type before placing.",
                        team_name(team),
                        if blocked_gap { ", deployment gap" } else { "" }
                    )
                };
                world
                    .entity_mut(button)
                    .insert(AccessibleLabel::new(accessible));
                if team == Team::Heroes && !view.local && !mini {
                    let owner = formation.owner(rank).unwrap_or(0);
                    let owner_label = label(
                        world,
                        anchor,
                        &format!("{team:?} Rank {rank} Owner"),
                        format!("P{}", owner + 1),
                        UiTextRole::Supporting,
                    );
                    text_color(world, owner_label, owner_color(owner));
                }
            }
        }
        for placement in formation.placements(team) {
            let Some(actor) = actor(view, placement.actor) else {
                continue;
            };
            let width = actor.actor.footprint;
            let selected_actor = state.selection.is_some_and(|(t, _)| t == team)
                && selected(view, state) == Some(actor.id);
            let left = if team == Team::Heroes {
                7 - placement.rank - width
            } else {
                placement.rank - 1
            };
            let button = control(
                world,
                ground,
                if matches!(state.mode, ConstructorMode::Move(_)) {
                    format!("Moving Actor {} Art", placement.actor.0)
                } else {
                    format!("Select {team:?} Rank {}", placement.rank)
                },
                "",
                Action::Constructor(ConstructorAction::Select(team, placement.rank)),
                false,
            );
            if matches!(state.mode, ConstructorMode::Move(_)) {
                world
                    .entity_mut(button)
                    .remove::<(Button, bevy_gamekit::ui::UiAction, Action)>()
                    .insert((Pickable::IGNORE, bevy::ui::FocusPolicy::Pass));
            }
            // The art spans exactly its occupied ranks. Rank pads below remain individual places.
            let children = world
                .get::<Children>(button)
                .map(|c| c.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            for child in children {
                world.despawn(child);
            }
            world.entity_mut(button).remove::<UiSpacing>().insert(Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(f32::from(left) * 100.0 / 6.0),
                width: Val::Percent(f32::from(width) * 100.0 / 6.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                border: UiRect::bottom(Val::Px(if selected_actor { 3.0 } else { 0.0 })),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                flex_direction: FlexDirection::Column,
                ..default()
            });
            quiet(world, button, selected_actor);
            world
                .get_mut::<UiSkinOverrides>(button)
                .expect("board skin")
                .background = Some(Color::NONE);
            let owner = formation.owner(placement.rank).unwrap_or(0);
            world
                .entity_mut(button)
                .insert(AccessibleLabel::new(format!(
                    "{}, {}, {}",
                    actor.actor.name,
                    span(placement.rank, width),
                    if team == Team::Heroes {
                        format!("player {}", owner + 1)
                    } else {
                        "enemy".into()
                    }
                )));
            art(
                world,
                button,
                format!("Actor {} Constructor Art", actor.id.0),
                actor.actor.appearance,
                Vec2::new(
                    (metrics.logical_size.x - 72.0) / 12.0 * f32::from(width),
                    if mini { 68.0 } else { height - 58.0 * scale },
                ),
                false,
                if mini {
                    54.0
                } else if view.local {
                    46.0 * scale
                } else {
                    20.0 * scale
                },
            );
            if !view.local && !mini {
                let owner_label = label(
                    world,
                    button,
                    &format!("Actor {} Board Owner", actor.id.0),
                    if team == Team::Heroes {
                        format!("P{}", owner + 1)
                    } else {
                        "AI".into()
                    },
                    UiTextRole::Supporting,
                );
                if team == Team::Heroes && !view.local {
                    text_color(world, owner_label, owner_color(owner));
                }
            }
        }
        if let Some((selected_team, rank)) = state.selection {
            if selected_team == team {
                let preview = match state.mode {
                    ConstructorMode::Pick => state
                        .preset
                        .as_ref()
                        .and_then(|id| view.catalog.as_ref()?.actor_preset(id))
                        .map(|p| (p.appearance, p.footprint)),
                    ConstructorMode::Move(id) => {
                        actor(view, id).map(|a| (a.actor.appearance, a.actor.footprint))
                    }
                    _ => None,
                };
                if let Some((kind, width)) = preview {
                    let width = width.min(7 - rank);
                    let left = if team == Team::Heroes {
                        7 - rank - width
                    } else {
                        rank - 1
                    };
                    let ghost = column(
                        world,
                        ground,
                        "Placement Ghost",
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(f32::from(left) * 100.0 / 6.0),
                            width: Val::Percent(f32::from(width) * 100.0 / 6.0),
                            top: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexEnd,
                            border: UiRect::bottom(Val::Px(4.0)),
                            ..default()
                        },
                    );
                    world.entity_mut(ghost).insert((
                        Pickable::IGNORE,
                        BackgroundColor(Color::NONE),
                        BorderColor::all(if preview_error(view, state).is_some() {
                            Color::srgb(1.0, 0.45, 0.32)
                        } else {
                            Color::srgb(0.67, 0.90, 0.71)
                        }),
                    ));
                    art(
                        world,
                        ghost,
                        "Candidate Placement Art".into(),
                        kind,
                        Vec2::new(
                            (metrics.logical_size.x - 72.0) / 12.0 * f32::from(width),
                            if mini { 68.0 } else { height - 58.0 * scale },
                        ),
                        true,
                        if mini { 54.0 } else { 20.0 * scale },
                    );
                    if !mini {
                        label(
                            world,
                            ghost,
                            "Preview Badge",
                            "PREVIEW",
                            UiTextRole::Supporting,
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn context(
    world: &mut World,
    parent: Entity,
    view: &LabyrinthView,
    state: &ConstructorState,
) {
    let Some((team, rank)) = state.selection else {
        let hint = label(
            world,
            parent,
            "Construction Advice",
            "Select a character to customize it, or an empty rank to choose who stands there.",
            UiTextRole::Body,
        );
        world.entity_mut(hint).insert(Node {
            margin: UiRect::vertical(Val::Px(16.0)),
            ..default()
        });
        return;
    };
    let tray = column(
        world,
        parent,
        "Construction Context",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(16.0)),
            flex_shrink: 0.0,
            border: UiRect::top(Val::Px(1.0)),
            ..default()
        },
    );
    world.entity_mut(tray).insert((
        BackgroundColor(Color::srgba(0.025, 0.045, 0.047, 0.94)),
        BorderColor::all(Color::srgba(0.68, 0.68, 0.53, 0.35)),
    ));
    let heading = row(world, tray, "Selected Place Header");
    let title = label(
        world,
        heading,
        "Selected Place",
        format!(
            "{} · {}",
            team_name(team),
            selected(view, state)
                .and_then(|id| actor(view, id))
                .map_or_else(
                    || span(rank, 1),
                    |a| format!("{} · {}", a.actor.name, span(rank, a.actor.footprint))
                )
        ),
        UiTextRole::Title,
    );
    world.entity_mut(title).insert(Node {
        flex_grow: 1.0,
        min_width: Val::Px(0.0),
        ..default()
    });
    if let Some(build) = state
        .preset
        .as_ref()
        .and_then(|id| view.catalog.as_ref()?.actor_preset(id))
        .and_then(|p| view.catalog.as_ref()?.resolve_build(&p.build).ok())
    {
        label(
            world,
            heading,
            "Move Detail Scroll Hint",
            format!(
                "{} {} · scroll / PgDn",
                build.moveset.skills.len(),
                if build.moveset.skills.len() == 1 {
                    "move"
                } else {
                    "moves"
                }
            ),
            UiTextRole::Supporting,
        );
    }
    control(
        world,
        heading,
        "Close Construction Context",
        if state.mode == ConstructorMode::Inspect {
            "Close"
        } else {
            "Cancel"
        },
        Action::Constructor(ConstructorAction::Close),
        false,
    );
    if team == Team::Heroes && !view.local {
        let owner = formation(view).owner(rank).unwrap_or(0);
        let owner_name = view
            .players
            .iter()
            .find(|p| p.slot == owner)
            .map_or("Host", |p| p.name.as_str());
        let ownership = row(world, tray, "Selected Place Ownership");
        let text = label(
            world,
            ownership,
            "Selected Place Owner",
            format!(
                "P{} · {} {}",
                owner + 1,
                owner_name,
                if selected(view, state).is_some() {
                    "controls this character"
                } else {
                    "chooses this reserved place"
                }
            ),
            UiTextRole::Supporting,
        );
        text_color(world, text, owner_color(owner));
        if view.host {
            control(
                world,
                ownership,
                "Assign Selected Place",
                "Assign player",
                Action::Constructor(ConstructorAction::Owners),
                false,
            );
        }
        if state.owners {
            let owners = row(world, tray, "Board Owner Choices");
            for player in view.players.iter().filter(|p| p.occupied) {
                control(
                    world,
                    owners,
                    format!("Assign Place To {}", player.slot),
                    format!("P{} · {}", player.slot + 1, player.name),
                    Action::Constructor(ConstructorAction::Assign {
                        rank,
                        owner: player.slot,
                        revision: view.setup_revision,
                    }),
                    player.slot == owner,
                );
            }
        }
    }
    if let Some(error) = preview_error(view, state) {
        let text = label(world, tray, "Placement Error", error, UiTextRole::Body);
        text_color(world, text, Color::srgb(1.0, 0.66, 0.48));
    }
    match state.mode {
        ConstructorMode::Inspect => {
            if let Some(id) = selected(view, state) {
                let actor = actor(view, id).expect("selected actor");
                label(
                    world,
                    tray,
                    "Selected Character Role",
                    format!(
                        "{} · {} HP · Speed {} · {}",
                        role(actor.actor.appearance),
                        actor.actor.max_hp,
                        actor.actor.base_speed,
                        actor
                            .actor
                            .build
                            .weapon
                            .as_ref()
                            .and_then(|id| view.catalog.as_ref()?.weapon(id))
                            .map_or("Unarmed", |weapon| weapon.name.as_str())
                    ),
                    UiTextRole::Body,
                );
                let actions = row(world, tray, "Selected Character Actions");
                control(
                    world,
                    actions,
                    format!("Edit Actor {}", id.0),
                    "Customize",
                    Action::Setup(setup::SetupAction::Edit(id)),
                    !can_choose(view, state),
                );
                control(
                    world,
                    actions,
                    "Replace Selected Actor",
                    "Replace…",
                    Action::Constructor(ConstructorAction::Pick),
                    !can_choose(view, state),
                );
                control(
                    world,
                    actions,
                    format!("Move Actor {}", id.0),
                    "Move…",
                    Action::Constructor(ConstructorAction::Move(id)),
                    !view.host || !view.admitted,
                );
                control(
                    world,
                    actions,
                    format!("Remove Actor {}", id.0),
                    "Remove",
                    Action::Constructor(ConstructorAction::Remove(id, view.setup_revision)),
                    !view.host || !view.admitted,
                );
                if let Some(catalog) = &view.catalog {
                    if let Ok(build) = catalog.resolve_build(&actor.actor.build) {
                        label(
                            world,
                            tray,
                            "Selected Character Moves",
                            format!(
                                "Moves · {}",
                                build
                                    .moveset
                                    .skills
                                    .iter()
                                    .map(|a| a.definition.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" · ")
                            ),
                            UiTextRole::Supporting,
                        );
                    }
                }
            } else {
                label(
                    world,
                    tray,
                    "Empty Place Instruction",
                    "This place is empty. Select it again to choose a character.",
                    UiTextRole::Body,
                );
            }
        }
        ConstructorMode::Pick => picker(world, tray, view, state, team, rank),
        ConstructorMode::Move(id) => {
            let Some(actor) = actor(view, id) else {
                return;
            };
            label(
                world,
                tray,
                "Movement Instruction",
                format!(
                    "Move {} · select a destination rank on the board. The complete {}-rank creature moves together.",
                    actor.actor.name, actor.actor.footprint
                ),
                UiTextRole::Body,
            );
            label(
                world,
                tray,
                "Movement Consequence",
                format!(
                    "Preview: {} → {}. Other characters stay in their places; the old position becomes empty.",
                    formation(view).rank(id).map_or_else(
                        || "Previous place".into(),
                        |r| span(r, actor.actor.footprint)
                    ),
                    span(rank, actor.actor.footprint)
                ),
                UiTextRole::Supporting,
            );
        }
    }
}

fn picker(
    world: &mut World,
    parent: Entity,
    view: &LabyrinthView,
    state: &ConstructorState,
    team: Team,
    rank: u8,
) {
    let Some(catalog) = &view.catalog else {
        return;
    };
    if state.preset.is_none() {
        label(
            world,
            parent,
            "Picker Instruction",
            "Choose a character type.",
            UiTextRole::Supporting,
        );
    }
    let compact_detail = compact(*world.resource::<ResolvedUiMetrics>()) && state.preset.is_some();
    if !compact_detail {
        let choices = column(
            world,
            parent,
            "Character Type Picker",
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                overflow: Overflow::scroll_x(),
                flex_shrink: 0.0,
                ..default()
            },
        );
        let scale = world.resource::<ResolvedUiMetrics>().content_scale;
        for preset in catalog
            .definition()
            .actor_presets
            .iter()
            .filter(|p| p.appearance.team() == team)
        {
            let active = state.preset.as_ref() == Some(&preset.id);
            let button = control(
                world,
                choices,
                format!("Inspect Type {}", preset.id),
                "",
                Action::Constructor(ConstructorAction::InspectType(preset.id.clone())),
                false,
            );
            let children = world
                .get::<Children>(button)
                .map(|c| c.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            for child in children {
                world.despawn(child);
            }
            world.entity_mut(button).remove::<UiSpacing>().insert(Node {
                width: Val::Px(174.0 * scale),
                min_width: Val::Px(0.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::bottom(Val::Px(if active { 3.0 } else { 1.0 })),
                ..default()
            });
            quiet(world, button, active);
            let image = crate::scene::portrait_image(world, preset.appearance);
            world.spawn((
                Name::new(format!("Type {} Portrait", preset.id)),
                image,
                Node {
                    width: Val::Px(48.0 * scale),
                    height: Val::Px(48.0 * scale),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
                ChildOf(button),
            ));
            label(world, button, "Type Name", &preset.name, UiTextRole::Body);
            world
                .entity_mut(button)
                .insert(AccessibleLabel::new(format!(
                    "Inspect {} · {} · {} ranks",
                    preset.name,
                    role(preset.appearance),
                    preset.footprint
                )));
        }
    }
    let Some(preset) = state
        .preset
        .as_ref()
        .and_then(|id| catalog.actor_preset(id))
    else {
        return;
    };
    let info = row(world, parent, "Inspected Type Summary");
    if compact_detail {
        control(
            world,
            info,
            "Back To Types",
            "Types",
            Action::Constructor(ConstructorAction::BrowseTypes),
            false,
        );
    }
    label(
        world,
        info,
        "Inspected Type",
        format!("{} · {}", preset.name, span(rank, preset.footprint)),
        UiTextRole::Title,
    );
    label(
        world,
        info,
        "Inspected Type Parameters",
        format!(
            "{} · {} HP · Speed {} · {}",
            role(preset.appearance),
            preset.max_hp,
            preset.base_speed,
            preset
                .build
                .weapon
                .as_ref()
                .and_then(|id| catalog.weapon(id))
                .map_or("Unarmed", |w| w.name.as_str())
        ),
        UiTextRole::Body,
    );
    if let Some(replaced) = selected(view, state).and_then(|id| actor(view, id)) {
        label(
            world,
            parent,
            "Replacement Consequence",
            format!(
                "Replaces {} and its build. Player ownership stays with this place. Other characters will not move.",
                replaced.actor.name
            ),
            UiTextRole::Supporting,
        );
    }
    if let Ok(build) = catalog.resolve_build(&preset.build) {
        let moves = column(
            world,
            parent,
            "Inspected Type Moves",
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                flex_shrink: 0.0,
                ..default()
            },
        );
        for skill in &build.moveset.skills {
            let facts = setup::details::move_facts(
                skill,
                (rank, rank.saturating_add(preset.footprint - 1)),
                catalog,
            );
            let can_act = (rank..rank.saturating_add(preset.footprint))
                .any(|r| skill.definition.allows_source_rank(r));
            let mut compact_facts = facts.clone();
            if let Some(effect) = compact_facts.first_mut() {
                *effect = format!(
                    "{effect} · {}",
                    if can_act {
                        "Usable here".to_owned()
                    } else {
                        format!("Unavailable at current rank {rank}")
                    }
                );
            }
            // Keep the decision's current-position consequence above long usage/provenance details.
            if compact_facts.len() > 2 {
                compact_facts.remove(2);
            }
            label(
                world,
                moves,
                &format!("Type Move {} Facts", skill.definition.id),
                format!("{} · {}", skill.definition.name, compact_facts.join("\n")),
                UiTextRole::Supporting,
            );
        }
    }
}

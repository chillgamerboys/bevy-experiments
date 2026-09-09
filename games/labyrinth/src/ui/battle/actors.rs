//! Stable actor/status entities, formation projection and local feedback visuals.

use super::*;

pub(super) fn formation(
    world: &mut World,
    parent: Entity,
    name: &str,
    title: &str,
    stacked: bool,
) -> Entity {
    let team = column(
        world,
        parent,
        name,
        Node {
            width: Val::Percent(if stacked { 100.0 } else { 49.0 }),
            min_width: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    label(
        world,
        team,
        &format!("{name} Heading"),
        title,
        UiTextRole::Supporting,
    );
    column(
        world,
        team,
        &format!("{name} Ranks"),
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Stretch,
            ..default()
        },
    )
}

pub(super) fn mount_actor(world: &mut World, parent: Entity, actor: &ActorSnapshot) {
    let entity = column(
        world,
        parent,
        &format!("Actor {} Tile", actor.id.0),
        Node {
            width: Val::Percent(24.0),
            min_width: Val::Px(44.0),
            min_height: Val::Px(150.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    let control = world
        .spawn((
            bevy_game_ui::button(format!("Actor {}", actor.id.0)),
            UiSkin::Card,
            bevy_game_ui::UiFocusId::new("labyrinth-actors", actor.id.0.to_string()),
            Action::Actor(actor.id),
            ChildOf(entity),
        ))
        .id();
    world.entity_mut(control).insert(Node {
        width: Val::Percent(100.0),
        min_width: Val::Px(44.0),
        min_height: Val::Px(150.0),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.0),
        padding: UiRect::all(Val::Px(10.0)),
        border: UiRect::all(Val::Px(2.0)),
        ..default()
    });
    portrait(world, control, actor.kind);
    let text = label(
        world,
        control,
        &format!("Actor {} Summary", actor.id.0),
        actor.name(),
        UiTextRole::Body,
    );
    let hp = column(
        world,
        control,
        &format!("Actor {} HP Track", actor.id.0),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(8.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world
        .entity_mut(hp)
        .insert(BackgroundColor(Color::srgb(0.025, 0.035, 0.04)));
    let bar = column(
        world,
        hp,
        &format!("Actor {} HP Bar", actor.id.0),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    );
    world
        .entity_mut(bar)
        .insert(BackgroundColor(actor_color(actor.kind)));
    // Status controls are siblings, not nested buttons, so each has one activation owner.
    let statuses = column(
        world,
        entity,
        &format!("Actor {} Statuses", actor.id.0),
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
    );
    world.entity_mut(entity).insert(ActorTile {
        actor: actor.id,
        control,
        text,
        bar,
        statuses,
        last_hp: actor.hp,
        flash_until: 0.0,
    });
}

pub(super) fn portrait(world: &mut World, parent: Entity, kind: ActorKind) {
    let portrait = column(
        world,
        parent,
        "Original Geometric Portrait",
        Node {
            width: Val::Px(74.0),
            height: Val::Px(74.0),
            flex_shrink: 0.0,
            align_self: AlignSelf::Center,
            ..default()
        },
    );
    world.entity_mut(portrait).insert(Portrait);
    let color = actor_color(kind);
    let dark = Color::srgb(0.04, 0.055, 0.06);
    let piece = |world: &mut World,
                 name: &str,
                 left: f32,
                 top: f32,
                 width: f32,
                 height: f32,
                 color: Color| {
        let entity = column(
            world,
            portrait,
            name,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(width),
                height: Val::Px(height),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
        );
        world.entity_mut(entity).insert(BackgroundColor(color));
    };
    piece(world, "Shoulders", 12.0, 41.0, 50.0, 28.0, color);
    piece(world, "Hood", 22.0, 12.0, 30.0, 35.0, color);
    piece(world, "Face Shadow", 28.0, 25.0, 18.0, 20.0, dark);
    piece(
        world,
        "Eyes",
        29.0,
        29.0,
        16.0,
        3.0,
        Color::srgb(0.94, 0.83, 0.53),
    );
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => {
            piece(world, "Helmet", 18.0, 9.0, 38.0, 15.0, color);
            piece(
                world,
                "Shield",
                4.0,
                43.0,
                23.0,
                30.0,
                Color::srgb(0.30, 0.43, 0.51),
            );
        }
        ActorKind::Hero(HeroClass::Knifehand) => {
            piece(
                world,
                "Scarf",
                18.0,
                40.0,
                37.0,
                8.0,
                Color::srgb(0.60, 0.27, 0.18),
            );
            piece(
                world,
                "Knife",
                62.0,
                39.0,
                4.0,
                28.0,
                Color::srgb(0.83, 0.85, 0.79),
            );
        }
        ActorKind::Hero(HeroClass::Scout) => {
            piece(
                world,
                "Quiver",
                59.0,
                14.0,
                7.0,
                49.0,
                Color::srgb(0.39, 0.45, 0.28),
            );
            piece(world, "Hood Peak", 31.0, 5.0, 11.0, 10.0, color);
        }
        ActorKind::Hero(HeroClass::FieldMedic) => {
            piece(
                world,
                "Medicine Cross Horizontal",
                30.0,
                53.0,
                15.0,
                4.0,
                dark,
            );
            piece(
                world,
                "Medicine Cross Vertical",
                35.0,
                48.0,
                4.0,
                14.0,
                dark,
            );
        }
        ActorKind::Enemy(_) => {
            piece(world, "Left Horn", 17.0, 4.0, 9.0, 20.0, color);
            piece(world, "Right Horn", 48.0, 4.0, 9.0, 20.0, color);
        }
    }
}

pub(super) fn actor_color(kind: ActorKind) -> Color {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => Color::srgb(0.50, 0.66, 0.73),
        ActorKind::Hero(HeroClass::Knifehand) => Color::srgb(0.82, 0.59, 0.40),
        ActorKind::Hero(HeroClass::Scout) => Color::srgb(0.50, 0.75, 0.60),
        ActorKind::Hero(HeroClass::FieldMedic) => Color::srgb(0.76, 0.69, 0.86),
        ActorKind::Enemy(_) => Color::srgb(0.74, 0.38, 0.33),
    }
}

pub(super) fn reorder(
    world: &mut World,
    parent: Entity,
    roster: &[ActorId],
    tiles: &BTreeMap<ActorId, Entity>,
) {
    let mut wanted = roster
        .iter()
        .filter_map(|actor| tiles.get(actor).copied())
        .collect::<Vec<_>>();
    let current = world
        .get::<Children>(parent)
        .map(|children| children.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    // Keep defeated entities parented (but hidden) for stable identity and recursive cleanup.
    for entity in &current {
        if !wanted.contains(entity) {
            wanted.push(*entity);
        }
    }
    if current != wanted {
        world.entity_mut(parent).replace_children(&wanted);
    }
}

pub(super) fn sync_statuses(world: &mut World, parent: Entity, actor: &ActorSnapshot) {
    let existing = world
        .query::<(Entity, &StatusBadge)>()
        .iter(world)
        .filter(|(_, badge)| badge.actor == actor.id)
        .map(|(entity, badge)| (badge.instance, (entity, badge.text)))
        .collect::<BTreeMap<_, _>>();
    for (instance, (entity, _)) in &existing {
        if !actor.statuses.iter().any(|status| status.id == *instance) {
            world.despawn(*entity);
        }
    }
    for status in &actor.statuses {
        let definition = status_definition(status.kind);
        let value = format!(
            "{} {} | {} left",
            definition.name, status.potency, status.remaining
        );
        if let Some((_, text)) = existing.get(&status.id) {
            set_text(world, *text, value);
        } else {
            let entity = control(
                world,
                parent,
                format!("Status {} {}", actor.id.0, status.id),
                value,
                Action::Status(actor.id, status.id),
                false,
            );
            if let Some(mut node) = world.get_mut::<Node>(entity) {
                node.min_width = Val::Px(44.0);
                node.padding = UiRect::all(Val::Px(4.0));
            }
            if let Some(text) = world
                .get::<Children>(entity)
                .and_then(|children| children.first())
                .copied()
            {
                world.entity_mut(entity).insert(StatusBadge {
                    actor: actor.id,
                    instance: status.id,
                    text,
                });
            }
        }
    }
}

pub(super) fn present(
    world: &mut World,
    view: &LabyrinthView,
    ui: &UiState,
    metrics: ResolvedUiMetrics,
    snapshot: &CombatSnapshot,
    tiles: &BTreeMap<ActorId, Entity>,
    time: f64,
) {
    let reduced_motion = world.resource::<UiMotionPreference>().reduced;
    for actor in &snapshot.actors {
        let Some(entity) = tiles.get(&actor.id).copied() else {
            continue;
        };
        let (control, text, bar, statuses, flash) = {
            let Some(mut tile) = world.get_mut::<ActorTile>(entity) else {
                continue;
            };
            if tile.last_hp != actor.hp {
                tile.flash_until = if reduced_motion { time } else { time + 0.35 };
                tile.last_hp = actor.hp;
            }
            (
                tile.control,
                tile.text,
                tile.bar,
                tile.statuses,
                tile.flash_until > time,
            )
        };
        let rank = snapshot
            .rank(actor.id)
            .map_or_else(|| "OUT".to_owned(), |rank| format!("RANK {rank}"));
        let state = if !actor.standing() {
            if actor.team() == Team::Heroes {
                "DOWNED"
            } else {
                "DEFEATED"
            }
        } else if snapshot.active_actor == Some(actor.id) {
            "ACTING"
        } else if ui.target == Some(actor.id) {
            "TARGET"
        } else {
            ""
        };
        let owner = match actor.kind {
            ActorKind::Hero(hero) => {
                if view.local {
                    "Local".to_owned()
                } else {
                    view.players
                        .iter()
                        .find(|player| player.hero == hero)
                        .map_or_else(|| "Unclaimed".to_owned(), |player| player.name.clone())
                }
            }
            ActorKind::Enemy(_) => "Host AI".to_owned(),
        };
        let feedback = world
            .resource::<BattleNodes>()
            .feedback
            .get(&actor.id)
            .map_or("", |(message, _)| message.as_str())
            .to_owned();
        let summary = if metrics.viewport == UiViewportClass::Compact {
            let name = actor.name();
            let statuses = actor
                .statuses
                .iter()
                .map(|status| {
                    format!(
                        "{}:{}",
                        status_definition(status.kind).name,
                        status.remaining
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            let compact_state = if state == "ACTING" {
                "ACT"
            } else if state == "TARGET" {
                "TGT"
            } else {
                state
            };
            format!(
                "{} {name}\n{} / {} HP\n{compact_state} {statuses}\n{feedback}",
                snapshot.rank(actor.id).unwrap_or(0),
                actor.hp,
                actor.max_hp
            )
        } else {
            format!(
                "{rank} | {state}\n{}\n{} / {} HP\n{owner}\n{feedback}",
                actor.name(),
                actor.hp,
                actor.max_hp
            )
        };
        set_text(world, text, summary);
        if let Some(mut node) = world.get_mut::<Node>(bar) {
            let width = Val::Percent(f32::from(actor.hp) / f32::from(actor.max_hp.max(1)) * 100.0);
            if node.width != width {
                node.width = width;
            }
        }
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            let display = if actor.team() == Team::Enemies && !actor.standing() {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
        }
        let border = if flash {
            Color::srgb(1.0, 0.40, 0.28)
        } else if ui.target == Some(actor.id) {
            Color::srgb(0.94, 0.83, 0.47)
        } else if snapshot.active_actor == Some(actor.id) {
            Color::srgb(0.60, 0.85, 0.77)
        } else {
            actor_color(actor.kind)
        };
        let wanted = UiSkinOverrides {
            border: Some(border),
            background: Some(if ui.target == Some(actor.id) {
                Color::srgb(0.19, 0.19, 0.12)
            } else if snapshot.active_actor == Some(actor.id) {
                Color::srgb(0.075, 0.18, 0.16)
            } else {
                Color::srgb(0.065, 0.09, 0.10)
            }),
            ..default()
        };
        if world.get::<UiSkinOverrides>(control) != Some(&wanted) {
            world.entity_mut(control).insert(wanted);
        }
        sync_statuses(world, statuses, actor);
    }
}

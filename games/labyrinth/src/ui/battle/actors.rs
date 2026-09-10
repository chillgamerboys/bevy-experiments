//! Stable actor hit areas and compact overlays above a game-owned world scene.

use super::*;
use crate::presentation::{BattlePresentation, CombatDisclosure};
use bevy::input_focus::InputFocusVisible;
use labyrinth_rules::{Boundary, Effect, StatusInstance};

#[derive(Component)]
struct ForecastBar(Entity);

#[derive(Component)]
struct FormationCue(Entity);

pub(super) fn formation(
    world: &mut World,
    parent: Entity,
    name: &str,
    _title: &str,
    _stacked: bool,
) -> Entity {
    let team = column(
        world,
        parent,
        name,
        Node {
            width: Val::Percent(49.0),
            height: Val::Percent(100.0),
            min_width: Val::Px(0.0),
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    );
    column(
        world,
        team,
        &format!("{name} Ranks"),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
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
            flex_basis: Val::Px(0.0),
            height: Val::Percent(100.0),
            min_width: Val::Px(44.0),
            min_height: Val::Px(0.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
    );
    // Layout owns the full formation column; input owns only the fitted art.
    // Keeping these separate prevents empty air above a sprite from targeting it.
    let art_layout = world
        .spawn((
            Name::new(format!("Actor {} Art Layout", actor.id.0)),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(44.0),
                flex_basis: Val::Px(0.0),
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(entity),
        ))
        .id();
    let control = world
        .spawn((
            bevy_game_ui::button(format!("Actor {}", actor.id.0)),
            bevy_game_ui::UiFocusId::new("labyrinth-actors", actor.id.0.to_string()),
            crate::scene::SceneActorAnchor { actor: actor.id },
            crate::scene::SceneActorLayout(art_layout),
            crate::scene::SceneActorEmphasis::default(),
            Action::Actor(actor.id),
            ChildOf(art_layout),
        ))
        .id();
    world.entity_mut(control).insert(Node {
        position_type: PositionType::Absolute,
        width: Val::Px(44.0),
        height: Val::Px(44.0),
        min_width: Val::Px(44.0),
        min_height: Val::Px(44.0),
        border: UiRect::bottom(Val::Px(3.0)),
        ..default()
    });
    world
        .entity_mut(control)
        .insert(BackgroundColor(Color::NONE));
    let cue = label(
        world,
        art_layout,
        &format!("Actor {} Formation Cue", actor.id.0),
        "",
        UiTextRole::Body,
    );
    world.entity_mut(cue).insert((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            width: Val::Percent(100.0),
            ..default()
        },
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
    ));
    world.entity_mut(entity).insert(FormationCue(cue));
    let text = label(
        world,
        entity,
        &format!("Actor {} Summary", actor.id.0),
        "",
        UiTextRole::Body,
    );
    world.entity_mut(text).insert((
        // Bounded identity/current-HP strings fit all six-rank supported widths.
        // Keep native width-aware justification; NoWrap uses intrinsic shaping
        // width and left-aligns this block despite a centered paragraph setting.
        TextLayout::justify(Justify::Center),
        Node {
            width: Val::Percent(100.0),
            // Identity and current HP always occupy two lines. The final
            // semantic height is resolved below, independently of their values.
            height: Val::Px(48.0),
            min_width: Val::Px(0.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.03, 0.86)),
    ));
    let hp = column(
        world,
        entity,
        &format!("Actor {} HP Track", actor.id.0),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(6.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world
        .entity_mut(hp)
        .insert(BackgroundColor(Color::srgb(0.02, 0.025, 0.03)));
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
    let forecast = world
        .spawn((
            Name::new(format!("Actor {} HP Forecast", actor.id.0)),
            Node {
                position_type: PositionType::Absolute,
                height: Val::Percent(100.0),
                display: Display::None,
                ..default()
            },
            ChildOf(hp),
            BackgroundColor(Color::NONE),
        ))
        .id();
    world.entity_mut(entity).insert(ForecastBar(forecast));
    // Reserve a stable footer on all actors, even when no effects are present.
    let statuses = column(
        world,
        entity,
        &format!("Actor {} Statuses", actor.id.0),
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(44.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
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

pub(super) fn actor_color(kind: ActorKind) -> Color {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => Color::srgb(0.50, 0.66, 0.73),
        ActorKind::Hero(HeroClass::Knifehand) => Color::srgb(0.82, 0.59, 0.40),
        ActorKind::Hero(HeroClass::Scout) => Color::srgb(0.50, 0.75, 0.60),
        ActorKind::Hero(HeroClass::FieldMedic) => Color::srgb(0.76, 0.69, 0.86),
        ActorKind::Enemy(_) => Color::srgb(0.74, 0.38, 0.33),
    }
}

/// A compact identity, not a rank or class: movement never changes this label.
pub(super) fn token(snapshot: &CombatSnapshot, actor: &ActorSnapshot) -> String {
    let ordinal = snapshot
        .actors
        .iter()
        .filter(|other| other.team() == actor.team() && other.id < actor.id)
        .count()
        + 1;
    format!(
        "{}{ordinal}",
        if actor.team() == Team::Heroes {
            "H"
        } else {
            "E"
        }
    )
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
    // Defeated entities remain parented but hidden for identity and recursive cleanup.
    for entity in &current {
        if !wanted.contains(entity) {
            wanted.push(*entity);
        }
    }
    if current != wanted {
        world.entity_mut(parent).replace_children(&wanted);
    }
}

fn prominent_status(actor: &ActorSnapshot) -> Option<&StatusInstance> {
    actor.statuses.iter().min_by_key(|status| {
        let definition = status_definition(status.kind);
        let deals_damage = definition
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::StatusDamage(_)));
        (!deals_damage, definition.priority, status.id)
    })
}

fn compact_status(actor: &ActorSnapshot, status: &StatusInstance) -> String {
    let definition = status_definition(status.kind);
    let abbreviation = definition.name.chars().take(3).collect::<String>();
    let others = actor.statuses.len().saturating_sub(1);
    let overflow = if others > 0 {
        format!("+{others}")
    } else {
        String::new()
    };
    let clock = match definition.duration.boundary {
        Boundary::OwnerTurnStart | Boundary::OwnerTurnEnd => "t",
        Boundary::RoundEnd => "r",
    };
    format!(
        "{abbreviation}{}\n{}{clock}{overflow}",
        status.potency, status.remaining
    )
}

fn status_accessibility(actor: &ActorSnapshot) -> String {
    let mut value = format!(
        "Inspect all {} effects on {}.",
        actor.statuses.len(),
        actor.name()
    );
    for status in &actor.statuses {
        let definition = status_definition(status.kind);
        let clock = match definition.duration.boundary {
            Boundary::OwnerTurnStart => "bearer turn starts",
            Boundary::OwnerTurnEnd => "bearer turn ends",
            Boundary::RoundEnd => "round ends",
        };
        value.push_str(&format!(
            " {}: potency {}, {} {clock} remaining.",
            definition.name, status.potency, status.remaining
        ));
    }
    value
}

pub(super) fn sync_statuses(world: &mut World, parent: Entity, actor: &ActorSnapshot) {
    let existing = world
        .query::<(Entity, &StatusBadge)>()
        .iter(world)
        .find(|(_, badge)| badge.actor == actor.id)
        .map(|(entity, badge)| (entity, badge.text));
    let Some(status) = prominent_status(actor) else {
        if let Some((entity, _)) = existing {
            if let Some(mut node) = world.get_mut::<Node>(entity) {
                if node.display != Display::None {
                    node.display = Display::None;
                }
            }
        }
        return;
    };
    let (entity, text) = existing.unwrap_or_else(|| {
        let entity = world
            .spawn((
                bevy_game_ui::button(format!("Actor {} Effects", actor.id.0)),
                bevy_game_ui::UiFocusId::new("labyrinth-effects", actor.id.0.to_string()),
                Action::Status(actor.id, status.id),
                ChildOf(parent),
            ))
            .id();
        world.entity_mut(entity).insert(Node {
            width: Val::Percent(100.0),
            min_width: Val::Px(44.0),
            min_height: Val::Px(44.0),
            height: Val::Percent(100.0),
            border: UiRect::bottom(Val::Px(2.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        });
        let text = label(
            world,
            entity,
            &format!("Actor {} Effects Summary", actor.id.0),
            "",
            UiTextRole::Supporting,
        );
        world
            .entity_mut(text)
            .insert(TextLayout::justify(Justify::Center));
        world.entity_mut(entity).insert(StatusBadge {
            actor: actor.id,
            instance: status.id,
            text,
        });
        (entity, text)
    });
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
    }
    if let Some(mut badge) = world.get_mut::<StatusBadge>(entity) {
        if badge.instance != status.id {
            badge.instance = status.id;
        }
    }
    if !matches!(world.get::<Action>(entity), Some(Action::Status(_, id)) if *id == status.id) {
        world
            .entity_mut(entity)
            .insert(Action::Status(actor.id, status.id));
    }
    set_text(world, text, compact_status(actor, status));
    let label = AccessibleLabel::new(status_accessibility(actor));
    if world.get::<AccessibleLabel>(entity).map(|value| &value.0) != Some(&label.0) {
        world.entity_mut(entity).insert(label);
    }
    let appearance = world
        .get_resource::<LabyrinthAppearance>()
        .cloned()
        .unwrap_or_default();
    let help = bevy_game_ui::UiContextHelp {
        title: format!("{} effects", actor.name()),
        body: status_accessibility(actor),
    };
    if world.get::<bevy_game_ui::UiContextHelp>(entity) != Some(&help) {
        world.entity_mut(entity).insert(help);
    }
    paint_marker(world, entity, appearance.accent);
    let backing = BackgroundColor(appearance.dock);
    if world.get::<BackgroundColor>(entity) != Some(&backing) {
        world.entity_mut(entity).insert(backing);
    }
}

fn paint_marker(world: &mut World, entity: Entity, resting: Color) {
    let appearance = world
        .get_resource::<LabyrinthAppearance>()
        .cloned()
        .unwrap_or_default();
    let focused = world.resource::<InputFocus>().get() == Some(entity)
        && world.resource::<InputFocusVisible>().0;
    let interacting = matches!(
        world.get::<Interaction>(entity),
        Some(Interaction::Hovered | Interaction::Pressed)
    );
    let color = if focused {
        appearance.accent
    } else if interacting {
        appearance.ink
    } else {
        resting
    };
    let border = BorderColor::all(color);
    if world.get::<BorderColor>(entity) != Some(&border) {
        world.entity_mut(entity).insert(border);
    }
}

fn summary_geometry(world: &mut World, entity: Entity, metrics: ResolvedUiMetrics) {
    let baseline = world.resource::<UiTheme>().body_size;
    let size = if baseline.is_finite() {
        (baseline * metrics.content_scale).max(18.0)
    } else {
        18.0
    };
    let line_height = (size * 1.2).ceil();
    let height = Val::Px(line_height * 2.0);
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        if node.height != height {
            node.height = height;
        }
    }
    let line_height = bevy::text::LineHeight::Px(line_height);
    if world.get::<bevy::text::LineHeight>(entity) != Some(&line_height) {
        world.entity_mut(entity).insert(line_height);
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
    let disclosure = world.resource::<CombatDisclosure>().clone();
    let projected = BattlePresentation::new(snapshot, &disclosure);
    let forecast = inspection::forecast_display(world, view, ui);
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    for actor in &snapshot.actors {
        let Some(facts) = projected.actor(actor.id) else {
            continue;
        };
        let Some(entity) = tiles.get(&actor.id).copied() else {
            continue;
        };
        let (control, text, bar, statuses, flash) = {
            let Some(mut tile) = world.get_mut::<ActorTile>(entity) else {
                continue;
            };
            if facts.health.as_known().is_some() && tile.last_hp != actor.hp {
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
        let identity = token(snapshot, actor);
        let owner = view.players.iter().find(|player| player.actor == actor.id);
        let yours = !view.local && owner.is_some_and(|player| Some(player.slot) == view.player);
        let ownership = if actor.team() == Team::Enemies {
            "Host-controlled enemy.".to_owned()
        } else if view.local {
            "Locally controlled hero.".to_owned()
        } else {
            owner.map_or_else(
                || "Unclaimed hero.".to_owned(),
                |player| {
                    format!(
                        "{}Owned by {}.",
                        if yours { "Your hero. " } else { "" },
                        player.name
                    )
                },
            )
        };
        // This is a preview of the same pure legality contract used by Confirm,
        // never an authorization gate: every actor remains inspectable off-turn.
        let eligible = !view.paused
            && view.admitted
            && ui.selected.is_some_and(|choice| {
                matches!(
                    choice,
                    Choice::Skill(_) | Choice::Reposition | Choice::Rescue
                ) && inspection::display_actor(view).is_some_and(|source| {
                    inspection::action_for(choice, Some(actor.id)).is_ok_and(|action| {
                        snapshot.validate_action_target(source.id, &action).is_ok()
                    })
                })
            });
        // The ownership asterisk is expanded in the accessible label and inspector.
        let after = forecast.as_ref().and_then(|forecast| {
            forecast
                .actors
                .iter()
                .find(|change| change.actor == actor.id)
        });
        // A prospective "34→27" must not add a wrapped line and lift the
        // character's sprite. Keep this compact numeric strip factual: the
        // forecast segment, action dock and accessible label show the projected
        // outcome without taking layout space from the actor's art anchor.
        let hp_text = facts
            .health
            .as_known()
            .map_or_else(|| "?".to_owned(), |health| health.current.to_string());
        set_text(
            world,
            text,
            format!("{identity}{}\n{hp_text}", if yours { "*" } else { "" }),
        );
        summary_geometry(world, text, metrics);
        let state = if !facts.standing {
            if actor.team() == Team::Heroes {
                "Downed"
            } else {
                "Defeated"
            }
        } else if snapshot.active_actor == Some(actor.id) {
            "Acting now"
        } else if ui.target == Some(actor.id) {
            "Selected target"
        } else {
            "Standing"
        };
        let health_text = facts.health.as_known().map_or_else(
            || "HP unknown".to_owned(),
            |health| format!("{} of {} HP", health.current, health.maximum),
        );
        let status_text = facts.statuses.as_known().map_or_else(
            || "Effects unknown".to_owned(),
            |statuses| format!("{} effects", statuses.len()),
        );
        let label = AccessibleLabel::new(format!(
            "{identity}, {}, {health_text}, rank {}. {state}. {ownership} {status_text}. {} Select or inspect. {}",
            actor.name(),
            snapshot.rank(actor.id).unwrap_or(0),
            if eligible {
                "Valid target for selected action."
            } else {
                ""
            },
            after.map_or("", |change| change.summary.as_str()),
        ));
        if world.get::<AccessibleLabel>(control).map(|value| &value.0) != Some(&label.0) {
            world
                .entity_mut(control)
                .insert(bevy_game_ui::UiContextHelp {
                    title: format!("{identity} · {}", actor.name()),
                    body: label.0.clone(),
                });
            world.entity_mut(control).insert(label);
        }
        let rank = snapshot.rank(actor.id).unwrap_or(0);
        let source_rank = actor.team() == Team::Heroes
            && matches!(ui.selected,
            Some(Choice::Skill(skill)) if rank > 0 && skill_definition(skill).source_ranks & (1 << (rank - 1)) != 0);
        if let Some(cue) = world.get::<FormationCue>(entity).map(|cue| cue.0) {
            // Keep position labels literal. Range/selection use the existing
            // footprint emphasis below, not unexplained punctuation.
            let text = rank.to_string();
            if let Some(mut label) = world.get_mut::<Text>(cue) {
                if label.0 != text {
                    label.0 = text;
                }
            }
        }
        if let Some(mut node) = world.get_mut::<Node>(control) {
            let width = if ui.target == Some(actor.id) {
                5.0
            } else if eligible || source_rank {
                3.0
            } else {
                1.0
            };
            if node.border.bottom != Val::Px(width) {
                node.border.bottom = Val::Px(width);
            }
        }
        if let Some(mut emphasis) = world.get_mut::<crate::scene::SceneActorEmphasis>(control) {
            let selected = ui.target == Some(actor.id);
            if emphasis.selected != selected {
                emphasis.selected = selected;
            }
        }
        if let Some(mut node) = world.get_mut::<Node>(bar) {
            let width = Val::Percent(facts.health.as_known().map_or(0.0, |health| {
                f32::from(health.current) / f32::from(health.maximum.max(1)) * 100.0
            }));
            if node.width != width {
                node.width = width;
            }
        }
        if let Some(forecast_bar) = world.get::<ForecastBar>(entity).map(|bar| bar.0) {
            let health_change = facts
                .health
                .as_known()
                .zip(after.and_then(|change| change.health.as_known()));
            let (left, width, color) =
                health_change.map_or((0.0, 0.0, Color::NONE), |(before, after)| {
                    let unit = 100.0 / f32::from(before.maximum.max(1));
                    (
                        f32::from(before.current.min(after.current)) * unit,
                        f32::from(before.current.abs_diff(after.current)) * unit,
                        if after.current < before.current {
                            appearance.damage
                        } else {
                            appearance.healing
                        },
                    )
                });
            if let Some(mut node) = world.get_mut::<Node>(forecast_bar) {
                let display = if width > 0.0 {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.left != Val::Percent(left)
                    || node.width != Val::Percent(width)
                    || node.display != display
                {
                    node.left = Val::Percent(left);
                    node.width = Val::Percent(width);
                    node.display = display;
                }
            }
            if world.get::<BackgroundColor>(forecast_bar) != Some(&BackgroundColor(color)) {
                world
                    .entity_mut(forecast_bar)
                    .insert(BackgroundColor(color));
            }
        }
        if world.get::<BackgroundColor>(text) != Some(&BackgroundColor(appearance.dock)) {
            world
                .entity_mut(text)
                .insert(BackgroundColor(appearance.dock));
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
        if let Some(mut node) = world.get_mut::<Node>(statuses) {
            // Two compact text lines can exceed the control minimum at 200%.
            let height = Val::Px((48.0 * metrics.content_scale).max(44.0 * metrics.control_scale));
            if node.height != height {
                node.height = height;
            }
        }
        let marker = if flash {
            appearance.damage
        } else if ui.target == Some(actor.id) {
            appearance.accent
        } else if snapshot.active_actor == Some(actor.id) {
            appearance.healing
        } else if eligible {
            appearance.positive
        } else {
            appearance.line
        };
        paint_marker(world, control, marker);
        let mut disclosed_actor = actor.clone();
        disclosed_actor.statuses = facts.statuses.as_known().cloned().unwrap_or_default();
        sync_statuses(world, statuses, &disclosed_actor);
        sync_unknown_status(world, statuses, actor, facts.statuses.as_known().is_none());
    }
}

#[derive(Component)]
struct UnknownEffects(ActorId);

fn sync_unknown_status(world: &mut World, parent: Entity, actor: &ActorSnapshot, unknown: bool) {
    let existing = world
        .query::<(Entity, &UnknownEffects)>()
        .iter(world)
        .find(|(_, e)| e.0 == actor.id)
        .map(|(entity, _)| entity);
    let entity = if let Some(entity) = existing {
        entity
    } else if unknown {
        let entity = world
            .spawn((
                bevy_game_ui::button(format!("Actor {} Unknown Effects", actor.id.0)),
                bevy_game_ui::UiFocusId::new("labyrinth-effects", actor.id.0.to_string()),
                bevy_game_ui::UiContextHelp {
                    title: "Unknown effects".to_owned(),
                    body: "Conditions have not been disclosed to this viewer.".to_owned(),
                },
                Action::InspectActor(actor.id),
                UnknownEffects(actor.id),
                ChildOf(parent),
            ))
            .id();
        world.entity_mut(entity).insert((
            AccessibleLabel::new("Effects unknown. Inspect actor."),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                min_height: Val::Px(44.0),
                min_width: Val::Px(44.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        label(world, entity, "Undisclosed Effects", "?", UiTextRole::Body);
        entity
    } else {
        return;
    };
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        let display = if unknown {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_game_test::{run_frames, HeadlessUiPlugin};
    use labyrinth_rules::{Combat, HeroSetup, StatusKind, DEFAULT_HERO_ROSTER};

    fn status(actor: ActorId, kind: StatusKind, id: u64) -> StatusInstance {
        let definition = status_definition(kind);
        StatusInstance {
            id,
            kind,
            bearer: actor,
            source: ActorId(101),
            potency: definition.potency,
            remaining: definition.duration.ticks,
            eligible_boundary: 1,
        }
    }

    fn overlay_world() -> World {
        let mut world = World::new();
        world.init_resource::<UiFonts>();
        world.init_resource::<InputFocus>();
        world.init_resource::<InputFocusVisible>();
        world
    }

    #[test]
    fn identity_tokens_ignore_rank_class_and_snapshot_array_order() {
        let setup = std::array::from_fn(|index| {
            HeroSetup::preset(
                ActorId(11 + u16::try_from(index).expect("six actors") * 7),
                HeroClass::Knifehand,
            )
        });
        let mut snapshot = Combat::with_heroes(42, setup)
            .expect("explicit IDs")
            .snapshot();
        let id = ActorId(18);
        assert_eq!(
            token(&snapshot, snapshot.actor(id).expect("second hero")),
            "H2"
        );
        snapshot.hero_formation.reverse();
        snapshot.actors.reverse();
        assert_eq!(
            token(&snapshot, snapshot.actor(id).expect("same hero")),
            "H2"
        );
        assert_eq!(
            token(
                &snapshot,
                snapshot.actor(ActorId(101)).expect("first enemy")
            ),
            "E1"
        );
    }

    #[test]
    fn effects_are_one_stable_control_with_exact_accessible_details() {
        let snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let mut actor = snapshot.actor(ActorId(1)).expect("hero").clone();
        actor.statuses = vec![
            status(actor.id, StatusKind::Brace, 700),
            status(actor.id, StatusKind::Bleed, 701),
        ];
        let mut world = overlay_world();
        let parent = world.spawn(Node::default()).id();
        sync_statuses(&mut world, parent, &actor);
        let (entity, text) = world
            .query::<(Entity, &StatusBadge)>()
            .iter(&world)
            .map(|(entity, badge)| (entity, badge.text))
            .next()
            .expect("one effects control");
        assert_eq!(world.query::<&StatusBadge>().iter(&world).count(), 1);
        assert_eq!(world.get::<Text>(text).expect("summary").0, "Ble2\n3t+1");
        assert_eq!(
            world.get::<AccessibleLabel>(entity).map(|value| &value.0),
            Some(&status_accessibility(&actor))
        );
        assert!(matches!(
            world.get::<Action>(entity),
            Some(Action::Status(ActorId(1), 701))
        ));
        actor.statuses.clear();
        sync_statuses(&mut world, parent, &actor);
        assert_eq!(
            world.get::<Node>(entity).expect("preserved").display,
            Display::None
        );
        actor
            .statuses
            .push(status(actor.id, StatusKind::Haste, 900));
        sync_statuses(&mut world, parent, &actor);
        assert_eq!(world.query::<&StatusBadge>().iter(&world).count(), 1);
        assert_eq!(
            world.get::<Node>(entity).expect("same control").display,
            Display::Flex
        );
        assert_eq!(world.get::<Text>(text).expect("summary").0, "Has3\n2r");
        assert!(matches!(
            world.get::<Action>(entity),
            Some(Action::Status(ActorId(1), 900))
        ));
    }

    #[test]
    fn native_actor_hit_area_is_unskinned_and_separate_from_hp_overlay() {
        let snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let actor = snapshot.actor(ActorId(1)).expect("hero");
        let mut world = overlay_world();
        let parent = world.spawn(Node::default()).id();
        mount_actor(&mut world, parent, actor);
        let tile = world
            .query::<&ActorTile>()
            .iter(&world)
            .next()
            .expect("tile");
        let control = tile.control;
        let text = tile.text;
        assert!(world.get::<Button>(control).is_some());
        assert!(world.get::<UiSkin>(control).is_none());
        assert_eq!(
            world.get::<BackgroundColor>(control),
            Some(&BackgroundColor(Color::NONE))
        );
        assert_eq!(
            world
                .get::<crate::scene::SceneActorAnchor>(control)
                .expect("art anchor")
                .actor,
            actor.id
        );
        assert_ne!(
            world.get::<ChildOf>(text).expect("summary parent").parent(),
            control
        );
        assert_eq!(
            world.get::<Node>(control).expect("target").min_height,
            Val::Px(44.0)
        );
        assert_eq!(
            world.get::<Node>(control).expect("target").min_width,
            Val::Px(44.0)
        );
        world
            .resource_mut::<InputFocus>()
            .set(control, bevy::input_focus::FocusCause::Navigated);
        world.resource_mut::<InputFocusVisible>().0 = true;
        paint_marker(&mut world, control, Color::NONE);
        assert_eq!(
            world.get::<BorderColor>(control),
            Some(&BorderColor::all(LabyrinthAppearance::default().accent))
        );
        assert_eq!(
            world.get::<BackgroundColor>(control),
            Some(&BackgroundColor(Color::NONE))
        );
    }

    fn present_overlay_fixture(app: &mut App, view: &LabyrinthView, ui: &UiState) {
        let tiles = app
            .world_mut()
            .query::<(Entity, &ActorTile)>()
            .iter(app.world())
            .map(|(entity, tile)| (tile.actor, entity))
            .collect();
        let metrics = *app.world().resource::<ResolvedUiMetrics>();
        present(
            app.world_mut(),
            view,
            ui,
            metrics,
            view.combat.as_ref().expect("fixture"),
            &tiles,
            0.0,
        );
        run_frames(app, 3);
    }

    fn art_geometry(app: &mut App) -> Vec<(ActorId, Vec2, Vec2)> {
        let mut geometry = app
            .world_mut()
            .query::<&ActorTile>()
            .iter(app.world())
            .map(|tile| {
                (
                    tile.actor,
                    app.world()
                        .get::<ComputedNode>(tile.control)
                        .expect("art layout")
                        .size(),
                    app.world()
                        .get::<UiGlobalTransform>(tile.control)
                        .expect("art position")
                        .translation,
                )
            })
            .collect::<Vec<_>>();
        geometry.sort_by_key(|(actor, _, _)| *actor);
        geometry
    }

    #[test]
    fn overlay_footprint_survives_forecasts_hp_ownership_and_disclosure_changes() {
        for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
            // Isolate actor-owned geometry from the command dock so this test
            // identifies accidental text-driven motion in the numeric footer.
            let mut app = App::new();
            app.add_plugins(HeadlessUiPlugin::new(1280, 720))
                .insert_resource(UiScalePreference(scale))
                .init_resource::<LabyrinthAppearance>()
                .init_resource::<CombatDisclosure>()
                .add_systems(Startup, crate::ui::load_default_font);
            app.finish();
            app.cleanup();
            let mut combat = Combat::new(42, DEFAULT_HERO_ROSTER).expect("fixture");
            // Reach this actor through real initiative transitions. Replacing
            // only active_actor would create an invalid snapshot, which the
            // forecast correctly rejects before producing accessible outcomes.
            for _ in 0..labyrinth_rules::MAX_ACTORS {
                let actor = combat.snapshot().active_actor.expect("active combat");
                if actor == ActorId(1) {
                    break;
                }
                combat.apply(actor, CombatAction::Wait).expect("wait turn");
            }
            let snapshot = combat.snapshot();
            assert_eq!(snapshot.active_actor, Some(ActorId(1)));
            snapshot.validate().expect("valid forecast input");
            let root = app
                .world_mut()
                .spawn(Node {
                    width: Val::Px(180.0),
                    height: Val::Px(440.0),
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .id();
            for id in [ActorId(1), ActorId(101)] {
                mount_actor(app.world_mut(), root, snapshot.actor(id).expect("actor"));
            }
            let mut view = LabyrinthView {
                local: true,
                admitted: true,
                combat: Some(snapshot),
                players: vec![crate::view::PlayerView {
                    slot: 0,
                    actor: ActorId(1),
                    hero: HeroClass::Gatekeeper,
                    name: "The hero's long player-owned display name".to_owned(),
                    occupied: true,
                    connected: true,
                    ready: true,
                }],
                ..default()
            };
            run_frames(&mut app, 3);
            let mut ui = UiState::default();
            present_overlay_fixture(&mut app, &view, &ui);
            let before = art_geometry(&mut app);
            ui.selected = Some(Choice::Skill(SkillId::FrontStrike));
            ui.target = Some(ActorId(101));
            present_overlay_fixture(&mut app, &view, &ui);
            assert_eq!(art_geometry(&mut app), before, "forecast moved art");
            let enemy = app
                .world_mut()
                .query::<&ActorTile>()
                .iter(app.world())
                .find(|tile| tile.actor == ActorId(101))
                .expect("enemy tile");
            let summary = app.world().get::<Text>(enemy.text).expect("HP text");
            assert!(!summary.0.contains('→'), "numeric strip stays factual");
            assert!(app
                .world()
                .get::<AccessibleLabel>(enemy.control)
                .expect("forecast accessibility")
                .0
                .contains('→'));
            for actor in &mut view.combat.as_mut().expect("snapshot").actors {
                if [ActorId(1), ActorId(101)].contains(&actor.id) {
                    actor.hp = 9;
                }
            }
            view.local = false;
            view.player = Some(0);
            present_overlay_fixture(&mut app, &view, &ui);
            assert_eq!(art_geometry(&mut app), before, "HP or ownership moved art");
            app.world_mut()
                .resource_mut::<CombatDisclosure>()
                .actors
                .insert(
                    ActorId(101),
                    crate::presentation::ActorDisclosure {
                        health: false,
                        ..default()
                    },
                );
            present_overlay_fixture(&mut app, &view, &ui);
            assert_eq!(art_geometry(&mut app), before, "unknown HP moved art");
            for tile in app.world_mut().query::<&ActorTile>().iter(app.world()) {
                let bounds = app
                    .world()
                    .get::<ComputedNode>(tile.text)
                    .expect("summary bounds");
                let text = app
                    .world()
                    .get::<bevy::text::TextLayoutInfo>(tile.text)
                    .expect("measured essential text");
                assert!(text.size.x <= bounds.size().x + 0.5);
                assert!(text.size.y <= bounds.size().y + 0.5);
                assert!(
                    app.world()
                        .get::<Text>(tile.text)
                        .expect("text")
                        .0
                        .lines()
                        .count()
                        == 2
                );
                assert_eq!(
                    app.world()
                        .get::<TextLayout>(tile.text)
                        .expect("layout")
                        .justify,
                    Justify::Center
                );
            }
        }
    }
}

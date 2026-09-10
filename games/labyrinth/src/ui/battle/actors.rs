//! Stable actor hit areas and compact overlays above a game-owned world scene.

use super::*;
use bevy::input_focus::InputFocusVisible;
use labyrinth_rules::{Boundary, Effect, StatusInstance};

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
    // Only this artwork hit area anchors the world sprite. Numeric overlays are
    // siblings below it, so text scaling never paints labels over character art.
    let control = world
        .spawn((
            bevy_game_ui::button(format!("Actor {}", actor.id.0)),
            bevy_game_ui::UiFocusId::new("labyrinth-actors", actor.id.0.to_string()),
            crate::scene::SceneActorAnchor { actor: actor.id },
            crate::scene::SceneActorEmphasis::default(),
            Action::Actor(actor.id),
            ChildOf(entity),
        ))
        .id();
    world.entity_mut(control).insert(Node {
        width: Val::Percent(100.0),
        min_width: Val::Px(44.0),
        min_height: Val::Px(44.0),
        flex_basis: Val::Px(0.0),
        flex_grow: 1.0,
        border: UiRect::bottom(Val::Px(3.0)),
        ..default()
    });
    world
        .entity_mut(control)
        .insert(BackgroundColor(Color::NONE));
    let text = label(
        world,
        entity,
        &format!("Actor {} Summary", actor.id.0),
        "",
        UiTextRole::Body,
    );
    world.entity_mut(text).insert((
        TextLayout::justify(Justify::Center),
        Node {
            width: Val::Percent(100.0),
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
    paint_marker(world, entity, Color::srgb(0.66, 0.55, 0.39));
    let backing = BackgroundColor(Color::srgba(0.02, 0.025, 0.03, 0.86));
    if world.get::<BackgroundColor>(entity) != Some(&backing) {
        world.entity_mut(entity).insert(backing);
    }
}

fn paint_marker(world: &mut World, entity: Entity, resting: Color) {
    let focused = world.resource::<InputFocus>().get() == Some(entity)
        && world.resource::<InputFocusVisible>().0;
    let interacting = matches!(
        world.get::<Interaction>(entity),
        Some(Interaction::Hovered | Interaction::Pressed)
    );
    let color = if focused {
        Color::srgb(1.0, 0.87, 0.47)
    } else if interacting {
        Color::srgb(0.95, 0.95, 0.88)
    } else {
        resting
    };
    let border = BorderColor::all(color);
    if world.get::<BorderColor>(entity) != Some(&border) {
        world.entity_mut(entity).insert(border);
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
                    inspection::action_for(choice, Some(actor.id))
                        .is_ok_and(|action| snapshot.validate_action(source.id, &action).is_ok())
                })
            });
        // The ownership asterisk is expanded in the accessible label and inspector.
        set_text(
            world,
            text,
            format!("{identity}{}\n{}", if yours { "*" } else { "" }, actor.hp),
        );
        let state = if !actor.standing() {
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
        let label = AccessibleLabel::new(format!(
            "{identity}, {}, {} of {} HP, rank {}. {state}. {ownership} {} effects. {} Select or inspect.",
            actor.name(),
            actor.hp,
            actor.max_hp,
            snapshot.rank(actor.id).unwrap_or(0),
            actor.statuses.len(),
            if eligible { "Valid target for selected action." } else { "" }
        ));
        if world.get::<AccessibleLabel>(control).map(|value| &value.0) != Some(&label.0) {
            world.entity_mut(control).insert(label);
        }
        if let Some(mut emphasis) = world.get_mut::<crate::scene::SceneActorEmphasis>(control) {
            let selected = ui.target == Some(actor.id);
            if emphasis.selected != selected {
                emphasis.selected = selected;
            }
        }
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
        if let Some(mut node) = world.get_mut::<Node>(statuses) {
            // Two compact text lines can exceed the control minimum at 200%.
            let height = Val::Px((48.0 * metrics.content_scale).max(44.0 * metrics.control_scale));
            if node.height != height {
                node.height = height;
            }
        }
        let marker = if flash {
            Color::srgb(1.0, 0.40, 0.28)
        } else if ui.target == Some(actor.id) {
            Color::srgb(0.94, 0.83, 0.47)
        } else if snapshot.active_actor == Some(actor.id) {
            Color::srgb(0.60, 0.85, 0.77)
        } else if eligible {
            Color::srgb(0.47, 0.72, 0.38)
        } else {
            Color::srgba(0.50, 0.56, 0.54, 0.30)
        };
        paint_marker(world, control, marker);
        sync_statuses(world, statuses, actor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            Some(&BorderColor::all(Color::srgb(1.0, 0.87, 0.47)))
        );
        assert_eq!(
            world.get::<BackgroundColor>(control),
            Some(&BackgroundColor(Color::NONE))
        );
    }
}

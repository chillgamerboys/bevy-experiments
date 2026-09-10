//! Compact rolled initiative identities; never predicts an unrolled next round.

use super::*;
use crate::presentation::CombatDisclosure;

#[derive(Component)]
struct InitiativePortrait {
    actor: ActorId,
    image: Entity,
    text: Entity,
}

pub(super) fn mount(world: &mut World, parent: Entity, snapshot: &CombatSnapshot) -> Entity {
    let row = column(
        world,
        parent,
        "Initiative Timeline",
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(66.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    );
    // Mount every stable identity, including actors absent from this round's roll.
    // A rescued hero can rejoin a later round without recreating the battlefield.
    for actor in &snapshot.actors {
        let control = world
            .spawn((
                bevy_game_ui::button(format!("Initiative Actor {}", actor.id.0)),
                bevy_game_ui::UiFocusId::new("labyrinth-initiative", actor.id.0.to_string()),
                Action::InspectActor(actor.id),
                ChildOf(row),
                UiSkin::Control,
                world.resource::<LabyrinthAppearance>().control(false),
            ))
            .id();
        world.entity_mut(control).insert(Node {
            flex_basis: Val::Px(0.0),
            flex_grow: 1.0,
            min_width: Val::Px(44.0),
            max_width: Val::Px(86.0),
            min_height: Val::Px(44.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            border: UiRect::bottom(Val::Px(2.0)),
            ..default()
        });
        let image = world
            .spawn((
                crate::scene::portrait_image(world, actor.kind),
                Node {
                    width: Val::Px(38.0),
                    height: Val::Px(38.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                ChildOf(control),
            ))
            .id();
        let text = label(
            world,
            control,
            &format!("Initiative Actor {} Label", actor.id.0),
            actors::token(snapshot, actor),
            UiTextRole::Supporting,
        );
        world.entity_mut(control).insert(InitiativePortrait {
            actor: actor.id,
            image,
            text,
        });
    }
    update(world, snapshot);
    row
}

pub(super) fn update(world: &mut World, snapshot: &CombatSnapshot) {
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let nodes = world
        .query::<(Entity, &InitiativePortrait)>()
        .iter(world)
        .map(|(e, n)| (e, n.actor, n.image, n.text))
        .collect::<Vec<_>>();
    let mut ordered = snapshot
        .initiative
        .iter()
        .filter_map(|entry| {
            nodes
                .iter()
                .find(|(_, id, _, _)| *id == entry.actor)
                .map(|n| n.0)
        })
        .collect::<Vec<_>>();
    // Keep inactive portraits parented to the battle root so they cannot become
    // orphaned focus targets, and so battle teardown owns their entire lifetime.
    for (entity, _, _, _) in &nodes {
        if !ordered.contains(entity) {
            ordered.push(*entity);
        }
    }
    if let Some(parent) = ordered
        .first()
        .and_then(|e| world.get::<ChildOf>(*e))
        .map(ChildOf::parent)
    {
        let current = world
            .get::<Children>(parent)
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        if current != ordered {
            world.entity_mut(parent).replace_children(&ordered);
        }
    }
    for (entity, id, image, text) in nodes {
        let Some(actor) = snapshot.actor(id) else {
            continue;
        };
        let included = snapshot.initiative.iter().any(|entry| entry.actor == id);
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            let display = if included {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        let active = snapshot.active_actor == Some(id);
        let completed = snapshot
            .initiative
            .iter()
            .find(|entry| entry.actor == id)
            .is_some_and(|entry| entry.completed);
        let yours = world.get_resource::<LabyrinthView>().is_some_and(|view| {
            !view.local
                && view
                    .players
                    .iter()
                    .any(|p| p.actor == id && Some(p.slot) == view.player)
        });
        let identity = actors::token(snapshot, actor);
        set_text(
            world,
            text,
            format!("{identity}{}", if yours { "*" } else { "" }),
        );
        let label = format!(
            "{identity}, {}. {}{} Inspect without changing target.",
            actor.name(),
            if yours { "Your hero. " } else { "" },
            if active {
                "Acting now."
            } else if completed {
                "Completed this round."
            } else {
                "Waiting this round."
            }
        );
        if world.get::<AccessibleLabel>(entity).map(|value| &value.0) != Some(&label) {
            world
                .entity_mut(entity)
                .insert(AccessibleLabel::new(label.clone()));
        }
        let help = bevy_game_ui::UiContextHelp {
            title: identity,
            body: label,
        };
        if world.get::<bevy_game_ui::UiContextHelp>(entity) != Some(&help) {
            world.entity_mut(entity).insert(help);
        }
        let paint = appearance.control(active);
        if world.get::<UiSkinOverrides>(entity) != Some(&paint) {
            world.entity_mut(entity).insert(paint);
        }
        let tint = if completed {
            appearance.muted
        } else {
            appearance.ink
        };
        if world.get::<TextColor>(text) != Some(&TextColor(tint)) {
            world.entity_mut(text).insert(TextColor(tint));
        }
        let mut portrait = crate::scene::portrait_image(world, actor.kind);
        portrait.color = if completed {
            Color::srgba(0.7, 0.7, 0.7, 0.6)
        } else {
            Color::WHITE
        };
        if !world.get::<ImageNode>(image).is_some_and(|existing| {
            existing.image == portrait.image
                && existing.rect == portrait.rect
                && existing.color == portrait.color
        }) {
            world.entity_mut(image).insert(portrait);
        }
    }
}

pub(super) fn disclosed_value(
    snapshot: &CombatSnapshot,
    disclosure: &CombatDisclosure,
    expanded: bool,
) -> String {
    if expanded && disclosure.has_unknown() {
        format!(
            "This round · initiative details undisclosed\n{}",
            value(snapshot, false)
        )
    } else {
        value(snapshot, expanded)
    }
}

pub(super) fn value(snapshot: &CombatSnapshot, expanded: bool) -> String {
    let entries = snapshot
        .initiative
        .iter()
        .map(|entry| {
            let identity = snapshot
                .actor(entry.actor)
                .map_or_else(|| "?".to_owned(), |actor| actors::token(snapshot, actor));
            let token = if entry.completed {
                format!("{identity}.")
            } else if snapshot.active_actor == Some(entry.actor) {
                format!("[{identity}]")
            } else {
                identity
            };
            if expanded {
                format!("{token} {}+{}={}", entry.speed, entry.roll, entry.total)
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(if expanded { "  /  " } else { "  " });
    if expanded {
        format!("This round: Speed + d8 | [acting] .completed\n{entries}")
    } else {
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labyrinth_rules::{Combat, DEFAULT_HERO_ROSTER};

    #[test]
    fn compact_order_keeps_all_rolled_actors_and_expands_only_on_request() {
        let mut snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let first = snapshot.initiative.first_mut().expect("first");
        first.completed = true;
        let first_actor = first.actor;
        let second = snapshot.initiative.get_mut(1).expect("second");
        second.completed = false;
        let second_actor = second.actor;
        let roll_text = format!("{}+{}={}", second.speed, second.roll, second.total);
        snapshot.active_actor = Some(second_actor);
        let compact = value(&snapshot, false);
        assert_eq!(compact.split_whitespace().count(), PARTY_SIZE * 2);
        assert!(!compact.contains("Speed"));
        assert!(!compact.contains('+'));
        let completed = snapshot.actor(first_actor).expect("actor");
        let active = snapshot.actor(second_actor).expect("actor");
        assert!(compact.contains(&format!("{}.", actors::token(&snapshot, completed))));
        assert!(compact.contains(&format!("[{}]", actors::token(&snapshot, active))));
        let expanded = value(&snapshot, true);
        assert!(expanded.contains("Speed + d8"));
        assert!(expanded.contains(&roll_text));
    }
}

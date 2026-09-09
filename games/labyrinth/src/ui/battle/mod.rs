//! Stable game-ID keyed battlefield and action inspection, independent of authority.

use std::collections::BTreeMap;

mod actors;
mod feedback;
mod inspection;
mod layout;
mod timeline;

use actors::{formation, mount_actor, reorder};
use inspection::slot_value;
pub(super) use inspection::{select_skill_slot, selected_action};
use layout::mount;

use super::*;
use bevy_game_ui::{UiRegionRole, UiViewportClass};
use labyrinth_rules::{
    skill_definition, status_definition, ActorKind, ActorSnapshot, CombatEventKind, CombatOutcome,
    CombatSnapshot, DamageKind, Team,
};

#[derive(Component)]
struct BattleRoot;
#[derive(Component)]
struct Portrait;

#[derive(Component)]
struct ActorTile {
    actor: ActorId,
    control: Entity,
    text: Entity,
    bar: Entity,
    statuses: Entity,
    last_hp: u16,
    flash_until: f64,
}

#[derive(Component)]
struct StatusBadge {
    actor: ActorId,
    instance: u64,
    text: Entity,
}

#[derive(Component, Clone, Copy)]
enum Slot {
    Hud,
    Timeline,
    Selected,
    Reason,
    Inspector,
    Log,
    Skill(usize),
}

#[derive(Resource)]
struct BattleNodes {
    formations: Entity,
    heroes: Entity,
    enemies: Entity,
    confirm: Entity,
    rematch: Entity,
    inspector: Entity,
    log: Entity,
    viewport: UiViewportClass,
    stacked: bool,
    last_event: Option<u64>,
    was_paused: bool,
    feedback: BTreeMap<ActorId, (String, f64)>,
}

pub(super) fn clear(world: &mut World) {
    despawn_marked::<BattleRoot>(world);
    world.remove_resource::<BattleNodes>();
}

fn text_slot(
    world: &mut World,
    parent: Entity,
    name: &str,
    slot: Slot,
    role: UiTextRole,
) -> Entity {
    let entity = label(world, parent, name, "", role);
    world.entity_mut(entity).insert(slot);
    entity
}

pub(super) fn present(
    world: &mut World,
    view: &LabyrinthView,
    ui: &mut UiState,
    metrics: ResolvedUiMetrics,
) {
    let Some(snapshot) = view.combat.as_ref() else {
        return;
    };
    if !world.contains_resource::<BattleNodes>() {
        mount(world, snapshot, metrics.viewport);
    }
    let decision = snapshot.active_actor.map(|actor| (snapshot.turn_id, actor));
    if ui.decision != decision || ui.encounter != Some(view.encounter) {
        ui.decision = decision;
        ui.encounter = Some(view.encounter);
        ui.selected = None;
        ui.target = None;
    }
    let time = world.resource::<Time>().elapsed_secs_f64();
    for mut portrait in world
        .query_filtered::<&mut Node, With<Portrait>>()
        .iter_mut(world)
    {
        let display = if metrics.viewport == UiViewportClass::Compact {
            Display::None
        } else {
            Display::Flex
        };
        if portrait.display != display {
            portrait.display = display;
        }
    }
    let tiles = world
        .query::<(Entity, &ActorTile)>()
        .iter(world)
        .map(|(entity, tile)| (tile.actor, entity))
        .collect::<BTreeMap<_, _>>();
    world.resource_scope(|world, mut nodes: Mut<BattleNodes>| {
        feedback::update(&mut nodes, view, time);
        let stacked = metrics.viewport == UiViewportClass::Compact && metrics.content_scale > 1.25;
        if nodes.viewport != metrics.viewport || nodes.stacked != stacked {
            nodes.viewport = metrics.viewport;
            nodes.stacked = stacked;
            if let Some(mut node) = world.get_mut::<Node>(nodes.formations) {
                node.flex_direction = if stacked {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                };
            }
            for rows in [nodes.heroes, nodes.enemies] {
                if let Some(parent) = world.get::<ChildOf>(rows).map(ChildOf::parent) {
                    if let Some(mut node) = world.get_mut::<Node>(parent) {
                        node.width = Val::Percent(if stacked { 100.0 } else { 49.0 });
                    }
                }
            }
        }
        // Both front ranks face the breach; presentation order is not actor identity.
        let facing_heroes = snapshot
            .hero_formation
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>();
        reorder(world, nodes.heroes, &facing_heroes, &tiles);
        reorder(world, nodes.enemies, &snapshot.enemy_formation, &tiles);
        set_disabled(world, nodes.confirm, selected_action(view, ui).is_err());
        set_disabled(
            world,
            nodes.rematch,
            !(view.host || view.local) || snapshot.outcome.is_none(),
        );
        if let Some(mut node) = world.get_mut::<Node>(nodes.rematch) {
            node.display = if snapshot.outcome.is_some() && (view.host || view.local) {
                Display::Flex
            } else {
                Display::None
            };
        }
        for (entity, shown) in [
            (nodes.inspector, !ui.show_log && ui.inspected.is_some()),
            (nodes.log, ui.show_log),
        ] {
            if let Some(mut node) = world.get_mut::<Node>(entity) {
                let display = if shown { Display::Flex } else { Display::None };
                if node.display != display {
                    node.display = display;
                }
            }
        }
    });
    actors::present(world, view, ui, metrics, snapshot, &tiles, time);
    inspection::paint_choices(world, view, ui);
    let slots = world
        .query::<(Entity, &Slot)>()
        .iter(world)
        .map(|(entity, slot)| (entity, *slot))
        .collect::<Vec<_>>();
    for (entity, slot) in slots {
        set_text(
            world,
            entity,
            slot_value(slot, view, ui, snapshot, metrics.viewport),
        );
    }
}

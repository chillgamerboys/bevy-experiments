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
    Order,
    Feedback,
    Selected,
    Reason,
    Inspector,
    Log,
    Skill(usize),
}

#[derive(Resource)]
struct BattleNodes {
    heroes: Entity,
    enemies: Entity,
    skills: Entity,
    loadout: Vec<SkillId>,
    confirm: Entity,
    rematch: Entity,
    inspector: Entity,
    log: Entity,
    drawer: Entity,
    drawer_body: Entity,
    order: Entity,
    viewport: UiViewportClass,
    last_event: Option<u64>,
    was_paused: bool,
    feedback: BTreeMap<ActorId, (String, f64)>,
}

pub(super) fn clear(world: &mut World) {
    despawn_marked::<BattleRoot>(world);
    world.remove_resource::<BattleNodes>();
}

pub(super) fn scroll_details(world: &mut World, direction: i8) {
    let Some(entity) = world
        .get_resource::<BattleNodes>()
        .map(|nodes| nodes.drawer_body)
    else {
        return;
    };
    let Some(node) = world.get::<ComputedNode>(entity) else {
        return;
    };
    let height = node.size().y * node.inverse_scale_factor;
    let maximum = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
    let current = world
        .get::<ScrollPosition>(entity)
        .map_or(0.0, |position| position.0.y);
    world.entity_mut(entity).insert(ScrollPosition(Vec2::new(
        0.0,
        (current + f32::from(direction) * height * 0.85).clamp(0.0, maximum),
    )));
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
    let tiles = world
        .query::<(Entity, &ActorTile)>()
        .iter(world)
        .map(|(entity, tile)| (tile.actor, entity))
        .collect::<BTreeMap<_, _>>();
    world.resource_scope(|world, mut nodes: Mut<BattleNodes>| {
        let loadout =
            inspection::display_actor(view).map_or_else(Vec::new, |actor| actor.skills().to_vec());
        if nodes.loadout != loadout {
            layout::mount_skills(world, nodes.skills, &loadout);
            nodes.loadout = loadout;
            // A loadout replacement must not leave an unequipped ability selected.
            if matches!(ui.selected, Some(Choice::Skill(skill)) if !nodes.loadout.contains(&skill))
            {
                ui.selected = None;
                ui.target = None;
            }
        }
        feedback::update(&mut nodes, view, time);
        nodes.viewport = metrics.viewport;
        if let Some(mut node) = world.get_mut::<Node>(nodes.drawer) {
            let width = Val::Percent(if metrics.content_scale > 1.25 {
                84.0
            } else {
                48.0
            });
            if node.width != width {
                node.width = width;
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
            (
                nodes.drawer,
                ui.show_inspector || ui.show_log || ui.show_timeline,
            ),
            (nodes.inspector, ui.show_inspector),
            (nodes.log, ui.show_log),
            (nodes.order, ui.show_timeline),
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
    let feedback = world
        .resource::<BattleNodes>()
        .feedback
        .iter()
        .map(|(actor, (message, _))| {
            format!(
                "{} {message}",
                snapshot
                    .actor(*actor)
                    .map_or_else(String::new, |actor| actors::token(snapshot, actor))
            )
        })
        .collect::<Vec<_>>()
        .join("   ·   ");
    for (entity, slot) in slots {
        let value = if matches!(slot, Slot::Feedback) {
            feedback.clone()
        } else {
            slot_value(slot, view, ui, snapshot, metrics.viewport)
        };
        set_text(world, entity, value);
    }
}

//! Stable game-ID keyed battlefield and action inspection, independent of authority.

use std::collections::BTreeMap;

mod actors;
mod dock;
mod feedback;
mod history;
pub(super) use history::scroll as scroll_history;
mod inspection;
mod layout;
mod timeline;
mod tooltips;
pub(super) use tooltips::{actor_subject, effects_subject};

use actors::{formation, mount_actor, reorder};
use inspection::slot_value;
pub(super) use inspection::{select_skill_slot, selected_action, skills_disclosed};
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
    Feedback,
    Reason,
}

#[derive(Resource)]
struct BattleNodes {
    heroes: Entity,
    enemies: Entity,
    skills: Entity,
    loadout: Vec<SkillId>,
    confirm: Entity,
    rematch: Entity,
    dock: dock::DockNodes,
    viewport: UiViewportClass,
    last_event: Option<u64>,
    was_paused: bool,
    feedback: BTreeMap<ActorId, (String, f64)>,
}

pub(super) fn clear(world: &mut World) {
    despawn_marked::<BattleRoot>(world);
    world.remove_resource::<BattleNodes>();
    world
        .resource_mut::<bevy_game_ui::UiTooltipCatalog>()
        .0
        .retain(|key, _| !key.0.starts_with("labyrinth/"));
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
    tooltips::refresh(world, view, ui);
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
        let presentation = crate::presentation::BattlePresentation::new(
            snapshot,
            world.resource::<crate::presentation::CombatDisclosure>(),
        );
        let loadout = inspection::display_actor(view)
            .and_then(|actor| presentation.actor(actor.id))
            .and_then(|actor| actor.details.as_known())
            .map_or_else(Vec::new, |details| details.skills.clone());
        if nodes.loadout != loadout {
            dock::mount_skills(world, nodes.skills, &loadout);
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
        dock::update(world, &nodes.dock, view, ui);
    });
    actors::present(world, view, ui, metrics, snapshot, &tiles, time);
    timeline::update(world, snapshot);
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
    let disclosure = world
        .resource::<crate::presentation::CombatDisclosure>()
        .clone();
    for (entity, slot) in slots {
        let value = if matches!(slot, Slot::Feedback) {
            if disclosure.has_unknown() {
                String::new()
            } else {
                feedback.clone()
            }
        } else {
            slot_value(slot, view, ui, snapshot, &disclosure)
        };
        set_text(world, entity, value);
    }
    history::present(world, view, ui);
}

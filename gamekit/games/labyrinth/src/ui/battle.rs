//! Stable game-ID keyed battlefield and action inspection, independent of authority.

use std::collections::BTreeMap;

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
    rail: Entity,
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

pub(super) fn select_skill_slot(view: &LabyrinthView, ui: &mut UiState, index: usize) {
    ui.inspected_status = None;
    if let Some(actor) = display_actor(view) {
        if let Some(skill) = actor.skills().get(index) {
            ui.selected = Some(Choice::Skill(*skill));
        }
    }
}

fn display_actor(view: &LabyrinthView) -> Option<&ActorSnapshot> {
    let snapshot = view.combat.as_ref()?;
    if view.local {
        snapshot
            .active_actor
            .and_then(|id| snapshot.actor(id))
            .filter(|actor| actor.team() == Team::Heroes)
            .or_else(|| {
                snapshot
                    .actors
                    .iter()
                    .find(|actor| actor.team() == Team::Heroes)
            })
    } else {
        let player = view
            .players
            .iter()
            .find(|player| Some(player.slot) == view.player)?;
        snapshot
            .actors
            .iter()
            .find(|actor| actor.kind == ActorKind::Hero(player.hero))
    }
}

fn action_for(choice: Choice, target: Option<ActorId>) -> Result<CombatAction, String> {
    let target = || target.ok_or_else(|| "Choose an actor tile as the target.".to_owned());
    Ok(match choice {
        Choice::Skill(skill) => CombatAction::Skill {
            skill,
            target: target()?,
        },
        Choice::Reposition => CombatAction::Reposition { ally: target()? },
        Choice::Rescue => CombatAction::Rescue { ally: target()? },
        Choice::Defend => CombatAction::Defend,
        Choice::Wait => CombatAction::Wait,
    })
}

pub(super) fn selected_action(
    view: &LabyrinthView,
    ui: &UiState,
) -> Result<(ActorId, CombatAction), String> {
    if view.paused {
        return Err("The company is waiting for a disconnected player.".to_owned());
    }
    if !view.admitted {
        return Err("Admission is not complete.".to_owned());
    }
    let snapshot = view
        .combat
        .as_ref()
        .ok_or_else(|| "No encounter is active.".to_owned())?;
    if snapshot.outcome.is_some() {
        return Err("The encounter is complete.".to_owned());
    }
    if ui.encounter != Some(view.encounter)
        || ui.decision != snapshot.active_actor.map(|actor| (snapshot.turn_id, actor))
    {
        return Err("The decision changed. Inspect and select the action again.".to_owned());
    }
    let actor =
        display_actor(view).ok_or_else(|| "You do not own a hero in this company.".to_owned())?;
    if snapshot.active_actor != Some(actor.id) {
        return Err("Wait for your hero's initiative turn.".to_owned());
    }
    let choice = ui
        .selected
        .ok_or_else(|| "Inspect a skill or choose a universal action.".to_owned())?;
    let action = action_for(choice, ui.target)?;
    snapshot
        .validate_action(actor.id, &action)
        .map_err(|error| error.to_string())?;
    Ok((actor.id, action))
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

fn mount(world: &mut World, snapshot: &CombatSnapshot, viewport: UiViewportClass) {
    let stacked = viewport == UiViewportClass::Compact
        && world.resource::<ResolvedUiMetrics>().content_scale > 1.25;
    let root = world
        .spawn((
            bevy_game_ui::screen_root("Labyrinth Battlefield"),
            BattleRoot,
        ))
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(18.0)),
        row_gap: Val::Px(12.0),
        overflow: Overflow::clip(),
        ..default()
    });
    let hud = column(
        world,
        root,
        "Battle HUD",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::SpaceBetween,
            row_gap: Val::Px(8.0),
            column_gap: Val::Px(14.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(hud).insert(UiRegionRole::Hud);
    text_slot(world, hud, "Battle Status", Slot::Hud, UiTextRole::Body);
    control(
        world,
        hud,
        "Timeline Toggle",
        "Order",
        Action::ToggleTimeline,
        false,
    );
    control(
        world,
        hud,
        "Battle Settings",
        "Settings",
        Action::Settings,
        false,
    );
    control(
        world,
        hud,
        "Battle Log Toggle",
        "Details",
        Action::ToggleLog,
        false,
    );
    let scroll = column(
        world,
        root,
        "Battlefield Scroll",
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    );
    world.entity_mut(scroll).insert(UiRegionRole::ScrollList);
    text_slot(
        world,
        scroll,
        "Initiative Timeline",
        Slot::Timeline,
        UiTextRole::Supporting,
    );
    let formations = column(
        world,
        scroll,
        "Facing Formations",
        Node {
            width: Val::Percent(100.0),
            flex_direction: if stacked {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            row_gap: Val::Px(14.0),
            column_gap: Val::Px(22.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    let heroes = formation(
        world,
        formations,
        "Your Company",
        "YOUR COMPANY | front 1 > 4 back | scroll / Tab",
        stacked,
    );
    let enemies = formation(
        world,
        formations,
        "The Opposition",
        "THE OPPOSITION | ranks run front to back",
        stacked,
    );
    for actor in &snapshot.actors {
        mount_actor(
            world,
            if actor.team() == Team::Heroes {
                heroes
            } else {
                enemies
            },
            actor,
        );
    }
    let inspector = world
        .spawn((bevy_game_ui::panel("Actor Inspector"), ChildOf(scroll)))
        .id();
    text_slot(
        world,
        inspector,
        "Inspector Text",
        Slot::Inspector,
        UiTextRole::Supporting,
    );
    let log = world
        .spawn((
            bevy_game_ui::panel("Combat Log"),
            UiRegionRole::ActivityFeed,
            ChildOf(scroll),
        ))
        .id();
    text_slot(world, log, "Log Text", Slot::Log, UiTextRole::Supporting);
    let rail = column(
        world,
        root,
        "Combat Action Rail",
        Node {
            width: Val::Percent(100.0),
            max_height: Val::Percent(34.0),
            min_height: Val::Px(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(12.0)),
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(rail).insert((
        UiRegionRole::ActionRail,
        BackgroundColor(Color::srgb(0.055, 0.075, 0.085)),
    ));
    let skills = column(
        world,
        rail,
        "Hero Skills",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    for index in 0..4 {
        let entity = control(
            world,
            skills,
            format!("Skill {index}"),
            "Skill",
            Action::SkillSlot(index),
            false,
        );
        if let Some(text) = world
            .get::<Children>(entity)
            .and_then(|children| children.first())
            .copied()
        {
            world.entity_mut(text).insert(Slot::Skill(index));
        }
    }
    let universal = column(
        world,
        rail,
        "Universal Actions",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    for (key, title, choice) in [
        ("Reposition", "Move", Choice::Reposition),
        ("Rescue", "Rescue", Choice::Rescue),
        ("Defend", "Defend", Choice::Defend),
        ("Wait", "Wait", Choice::Wait),
    ] {
        control(world, universal, key, title, Action::Choice(choice), false);
    }
    let selected = text_slot(
        world,
        rail,
        "Selected Skill",
        Slot::Selected,
        UiTextRole::Body,
    );
    let reason = text_slot(
        world,
        rail,
        "Target Legality",
        Slot::Reason,
        UiTextRole::Supporting,
    );
    world
        .entity_mut(rail)
        .replace_children(&[selected, reason, skills, universal]);
    let commit = column(
        world,
        root,
        "Commit Actions",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    let confirm = control(
        world,
        commit,
        "Confirm Combat Action",
        "Confirm action",
        Action::Confirm,
        true,
    );
    control(
        world,
        commit,
        "Cancel Combat Selection",
        "Cancel selection",
        Action::Cancel,
        false,
    );
    let rematch = control(
        world,
        commit,
        "Combat Rematch",
        "Return party to lobby",
        Action::Rematch,
        true,
    );
    control(world, commit, "Combat Leave", "Menu", Action::Leave, false);
    world.insert_resource(BattleNodes {
        formations,
        heroes,
        enemies,
        rail,
        confirm,
        rematch,
        inspector,
        log,
        viewport,
        stacked,
        last_event: None,
        was_paused: false,
        feedback: BTreeMap::new(),
    });
}

fn formation(world: &mut World, parent: Entity, name: &str, title: &str, stacked: bool) -> Entity {
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

fn mount_actor(world: &mut World, parent: Entity, actor: &ActorSnapshot) {
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

fn portrait(world: &mut World, parent: Entity, kind: ActorKind) {
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

fn actor_color(kind: ActorKind) -> Color {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => Color::srgb(0.50, 0.66, 0.73),
        ActorKind::Hero(HeroClass::Knifehand) => Color::srgb(0.82, 0.59, 0.40),
        ActorKind::Hero(HeroClass::Scout) => Color::srgb(0.50, 0.75, 0.60),
        ActorKind::Hero(HeroClass::FieldMedic) => Color::srgb(0.76, 0.69, 0.86),
        ActorKind::Enemy(_) => Color::srgb(0.74, 0.38, 0.33),
    }
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
        let highest = view.events.last().map_or(0, |event| event.id);
        if nodes.last_event.is_none() || view.paused || !view.admitted || nodes.was_paused {
            // First snapshots and recovery are a baseline, not an animation replay.
            nodes.last_event = Some(highest);
            nodes.feedback.clear();
        } else {
            let previous = nodes.last_event.unwrap_or(0);
            let fresh = view
                .events
                .iter()
                .rev()
                .filter(|event| event.id > previous)
                .take(32)
                .collect::<Vec<_>>();
            for event in fresh.into_iter().rev() {
                let feedback = match event.event.kind {
                    CombatEventKind::Damage {
                        target,
                        amount,
                        kind,
                        ..
                    } => Some((
                        target,
                        format!(
                            "-{amount} {}",
                            if kind == DamageKind::Bleed {
                                "BLEED"
                            } else {
                                "HP"
                            }
                        ),
                    )),
                    CombatEventKind::Healed { target, amount, .. } => {
                        Some((target, format!("+{amount} HP")))
                    }
                    CombatEventKind::Rescued { actor, hp, .. } => {
                        Some((actor, format!("RESCUED +{hp}")))
                    }
                    _ => None,
                };
                if let Some((actor, message)) = feedback {
                    nodes.feedback.insert(actor, (message, time + 1.2));
                }
            }
            nodes.last_event = Some(highest.max(previous));
        }
        nodes.was_paused = view.paused || !view.admitted;
        nodes.feedback.retain(|_, (_, until)| *until > time);
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
        reorder(world, nodes.heroes, &snapshot.hero_formation, &tiles);
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
        for (entity, shown) in [(nodes.inspector, !ui.show_log), (nodes.log, ui.show_log)] {
            if let Some(mut node) = world.get_mut::<Node>(entity) {
                let display = if shown { Display::Flex } else { Display::None };
                if node.display != display {
                    node.display = display;
                }
            }
        }
        let _rail = nodes.rail;
    });
    for actor in &snapshot.actors {
        let Some(entity) = tiles.get(&actor.id).copied() else {
            continue;
        };
        let (control, text, bar, statuses, flash) = {
            let Some(mut tile) = world.get_mut::<ActorTile>(entity) else {
                continue;
            };
            if tile.last_hp != actor.hp {
                tile.flash_until = if ui.reduced_motion { time } else { time + 0.35 };
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
            let name = if actor.kind == ActorKind::Hero(HeroClass::FieldMedic) {
                "Medic"
            } else {
                actor.name()
            };
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
        if let Some(mut current) = world.get_mut::<BorderColor>(control) {
            let wanted = BorderColor::all(border);
            if *current != wanted {
                *current = wanted;
            }
        }
        sync_statuses(world, statuses, actor);
    }
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

fn reorder(
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

fn sync_statuses(world: &mut World, parent: Entity, actor: &ActorSnapshot) {
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

fn rank_mask(mask: u8) -> String {
    (1..=4)
        .map(|rank| {
            if mask & (1 << (rank - 1)) != 0 {
                rank.to_string()
            } else {
                "-".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn slot_value(
    slot: Slot,
    view: &LabyrinthView,
    ui: &UiState,
    snapshot: &CombatSnapshot,
    viewport: UiViewportClass,
) -> String {
    match slot {
        Slot::Hud => {
            let actor = snapshot
                .active_actor
                .and_then(|id| snapshot.actor(id))
                .map_or("No actor", ActorSnapshot::name);
            let ending = snapshot.outcome.map(|outcome| match outcome {
                CombatOutcome::Victory => "VICTORY",
                CombatOutcome::Defeat => "DEFEAT",
            });
            let mode = if view.local {
                "LOCAL | all four heroes".to_owned()
            } else {
                format!(
                    "CO-OP | {}/4 connected",
                    view.players
                        .iter()
                        .filter(|player| player.connected)
                        .count()
                )
            };
            if viewport == UiViewportClass::Compact {
                format!(
                    "{} | R{} | {}",
                    if view.local { "LOCAL" } else { "CO-OP" },
                    snapshot.round,
                    ending.unwrap_or(actor)
                )
            } else {
                format!(
                    "LABYRINTH | {mode}\nRound {} | {}",
                    snapshot.round,
                    ending.unwrap_or(actor)
                )
            }
        }
        Slot::Timeline => {
            if viewport == UiViewportClass::Compact && !ui.show_timeline {
                let next = snapshot
                    .initiative
                    .iter()
                    .filter(|entry| !entry.completed)
                    .take(2)
                    .map(|entry| {
                        format!(
                            "{} {}+{}={}",
                            snapshot.actor(entry.actor).map_or("?", ActorSnapshot::name),
                            entry.speed,
                            entry.roll,
                            entry.total
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | then ");
                return format!("INITIATIVE | {next}");
            }
            let entries = snapshot
                .initiative
                .iter()
                .map(|entry| {
                    let name = snapshot.actor(entry.actor).map_or("?", ActorSnapshot::name);
                    let mark = if entry.completed {
                        "done"
                    } else if snapshot.active_actor == Some(entry.actor) {
                        "NOW"
                    } else {
                        "next"
                    };
                    format!(
                        "{name} {}+{}={} [{mark}]",
                        entry.speed, entry.roll, entry.total
                    )
                })
                .collect::<Vec<_>>()
                .join("  /  ");
            format!("THIS ROUND | Speed + d8\n{entries}")
        }
        Slot::Selected => {
            if let Some((actor, id)) = ui.inspected_status {
                if let Some(status) = snapshot
                    .actor(actor)
                    .and_then(|actor| actor.statuses.iter().find(|status| status.id == id))
                {
                    return format!(
                        "{} | {} boundaries left. {}",
                        status_definition(status.kind).name,
                        status.remaining,
                        status_definition(status.kind).description
                    );
                }
            }
            match ui.selected {
            Some(Choice::Skill(skill)) => {
                let definition = skill_definition(skill);
                format!("{} | src [{}] tgt [{}]\n{}", definition.name, rank_mask(definition.source_ranks), rank_mask(definition.target_ranks), definition.description)
            }
            Some(Choice::Reposition) => "MOVE | swap with an adjacent ally, including a downed ally. Costs this turn.".to_owned(),
            Some(Choice::Rescue) => "RESCUE | revive any downed ally at 25% maximum HP. Costs this turn.".to_owned(),
            Some(Choice::Defend) => "DEFEND | Brace reduces direct damage by 2 until your next turn starts; not bleed.".to_owned(),
            Some(Choice::Wait) => "WAIT | spend this turn without another effect.".to_owned(),
            None => "Inspect a skill (1-4), select an actor tile, then Confirm. Inspection never spends a turn.".to_owned(),
        }
        }
        Slot::Reason => {
            let target = ui
                .target
                .and_then(|id| snapshot.actor(id))
                .map_or("none", ActorSnapshot::name);
            let reason = selected_action(view, ui).map_or_else(
                |reason| reason,
                |_| "Legal action | ready to confirm".to_owned(),
            );
            format!("Target: {target} | {reason}")
        }
        Slot::Inspector => {
            let Some(actor) = ui
                .inspected
                .and_then(|id| snapshot.actor(id))
                .or_else(|| display_actor(view))
            else {
                return "Choose an actor or a status badge to inspect it.".to_owned();
            };
            let mut value = format!("{} | {} / {} HP | Speed {}\nRank is position, not ownership. Heroes at 0 HP are downed and can be rescued.", actor.name(), actor.hp, actor.max_hp, actor.speed());
            for status in &actor.statuses {
                if ui.inspected_status.is_none_or(|(_, id)| id == status.id) {
                    let definition = status_definition(status.kind);
                    value.push_str(&format!(
                        "\n{} | potency {} | {} boundaries left. {}",
                        definition.name, status.potency, status.remaining, definition.description
                    ));
                }
            }
            value
        }
        Slot::Log => {
            if view.log.is_empty() {
                "No outcomes yet. Combat events appear here in host order.".to_owned()
            } else {
                view.log
                    .iter()
                    .rev()
                    .take(12)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        Slot::Skill(index) => display_actor(view)
            .and_then(|actor| actor.skills().get(index).map(|skill| (actor, *skill)))
            .map_or_else(
                || "No skill".to_owned(),
                |(actor, skill)| {
                    let selected = if ui.selected == Some(Choice::Skill(skill)) {
                        " | selected"
                    } else {
                        ""
                    };
                    let uses = actor
                        .remaining_uses(skill)
                        .map_or_else(String::new, |remaining| format!(" | {remaining} uses"));
                    format!(
                        "{}. {}{uses}{selected}",
                        index + 1,
                        skill_definition(skill).name
                    )
                },
            ),
    }
}

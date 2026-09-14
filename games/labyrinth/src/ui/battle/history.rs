//! Compact encounter history with a bounded mounted window over stable event IDs.
use super::*;
use crate::view::{EncounterHistory, PresentedEvent, HISTORY_PAGE_EVENTS};
use bevy_gamekit::ui::{
    UiFeedScroll, UiTextStyle, UiTooltipCatalog, UiTooltipContent, UiTooltipOpen, UiTooltipSource,
    UiTooltipState, UiTooltipSubject,
};

const ROW_HEIGHT: f32 = 34.0;
const OVERSCAN: u64 = 4;
const MOUNT_LIMIT: usize = 32;

/// Keep the log above actor artwork without changing battlefield geometry.
#[derive(Component, PartialEq)]
pub(super) struct HistorySafeBottom(pub f32);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    id: u64,
    text: String,
    heading: bool,
}

fn entry(view: &LabyrinthView, item: &PresentedEvent) -> Entry {
    let name = |id: ActorId| {
        view.combat
            .as_ref()
            .and_then(|snapshot| {
                snapshot
                    .actor(id)
                    .map(|actor| actors::display_name(snapshot, actor))
            })
            .unwrap_or_else(|| format!("Character {}", id.0))
    };
    let heading = matches!(
        item.event.kind,
        CombatEventKind::Action { .. }
            | CombatEventKind::TurnStarted { .. }
            | CombatEventKind::RoundStarted { .. }
    );
    let text = match &item.event.kind {
        CombatEventKind::Action { actor, action } => {
            let actor_snapshot = view
                .combat
                .as_ref()
                .and_then(|snapshot| snapshot.actor(*actor));
            let title = match *action {
                CombatAction::Skill { index, .. } => {
                    inspection::choice_title(actor_snapshot, Some(Choice::Skill(index)))
                }
                CombatAction::LegacySkill { skill, .. } => {
                    inspection::choice_title(actor_snapshot, Some(Choice::LegacySkill(skill)))
                }
                CombatAction::Reposition { .. } => "Move",
                CombatAction::Rescue { .. } => "Rescue",
                CombatAction::Defend => "Guard",
                CombatAction::Wait => "Wait",
            };
            let target = match *action {
                CombatAction::Skill { target, .. } | CombatAction::LegacySkill { target, .. } => {
                    Some(target)
                }
                CombatAction::Reposition { ally } | CombatAction::Rescue { ally } => Some(ally),
                CombatAction::Defend | CombatAction::Wait => None,
            };
            format!(
                "{} · {title}{}",
                name(*actor),
                target.map_or_else(String::new, |target| format!(" → {}", name(target)))
            )
        }
        CombatEventKind::RoundStarted { round } => format!("Round {round} · initiative rolled"),
        CombatEventKind::TurnStarted { actor, turn_id } => {
            format!("Turn {turn_id} · {}", name(*actor))
        }
        CombatEventKind::TurnSkipped { actor } => format!("{} cannot act", name(*actor)),
        CombatEventKind::Damage {
            source,
            target,
            amount,
            kind,
        } => format!(
            "{} → {} −{amount} HP{}",
            name(*source),
            name(*target),
            if *kind == DamageKind::Bleed {
                " · Bleed"
            } else {
                ""
            }
        ),
        CombatEventKind::Healed {
            source,
            target,
            amount,
        } => format!("{} → {} +{amount} HP", name(*source), name(*target)),
        CombatEventKind::Moved { actor, rank } => format!("{} → rank {rank}", name(*actor)),
        CombatEventKind::Downed { actor } => format!("{} is downed", name(*actor)),
        CombatEventKind::Defeated { actor } => format!("{} dies and leaves a corpse", name(*actor)),
        CombatEventKind::Rescued { source, actor, hp } => {
            format!("{} rescues {} · {hp} HP", name(*source), name(*actor))
        }
        CombatEventKind::DeathSave {
            actor,
            roll,
            failures,
        } => match roll {
            Some(roll) => format!(
                "{} · death save {roll}: {} · {failures}/3 failures",
                name(*actor),
                if *roll >= labyrinth_rules::DEATH_SAVE_TARGET {
                    "holds on"
                } else {
                    "failed"
                }
            ),
            None => format!("{} · hit while dying · {failures}/3 failures", name(*actor)),
        },
        CombatEventKind::CorpseRemoved { actor, expired } => format!(
            "{} · corpse {}",
            name(*actor),
            if *expired { "expired" } else { "destroyed" }
        ),
        CombatEventKind::StatusApplied { instance } => format!(
            "{} gains {}",
            name(instance.bearer),
            status_definition(instance.kind).name
        ),
        CombatEventKind::StatusRefreshed { instance } => format!(
            "{} · {} refreshed",
            name(instance.bearer),
            status_definition(instance.kind).name
        ),
        CombatEventKind::StatusTriggered { actor, kind, .. } => format!(
            "{} · {} triggers",
            name(*actor),
            status_definition(*kind).name
        ),
        CombatEventKind::StatusRemoved {
            actor,
            kind,
            reason,
            ..
        } => format!(
            "{} · {} removed ({reason:?})",
            name(*actor),
            status_definition(*kind).name
        ),
        CombatEventKind::Finished { outcome } => format!("{outcome:?}"),
    };
    Entry {
        id: item.id,
        text,
        heading,
    }
}

#[derive(Component)]
struct HistoryPanel;
#[derive(Component)]
struct HistoryBody;
#[derive(Component)]
struct HistoryLatest;
#[derive(Component)]
struct HistoryEncounter(u64);
#[derive(Component)]
struct HistorySpacer(bool);
#[derive(Component, PartialEq, Eq)]
struct HistoryRow(Entry);
#[derive(Component, Default)]
struct HistoryReading {
    row_height: f32,
    anchor: u64,
    fraction: f32,
}

pub(crate) fn scroll(world: &mut World, direction: i8) {
    let Some(entity) = world
        .query_filtered::<Entity, With<HistoryBody>>()
        .iter(world)
        .next()
    else {
        return;
    };
    if !bevy_gamekit::ui::activation_eligible(world, entity) {
        return;
    }
    let Some(node) = world.get::<ComputedNode>(entity) else {
        return;
    };
    let height = node.size().y * node.inverse_scale_factor;
    let maximum = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
    let current = world
        .get::<ScrollPosition>(entity)
        .map_or(0.0, |position| position.y);
    let next = match direction {
        100.. => maximum,
        ..=-100 => 0.0,
        _ => (current + f32::from(direction) * height * 0.85).clamp(0.0, maximum),
    };
    world
        .entity_mut(entity)
        .insert(ScrollPosition(Vec2::new(0.0, next)));
}

pub(crate) fn latest(world: &mut World) {
    let Some(body) = world
        .query_filtered::<Entity, With<HistoryBody>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let maximum = world.get::<ComputedNode>(body).map_or(0.0, |node| {
        ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0)
    });
    if let Some(mut feed) = world.get_mut::<UiFeedScroll>(body) {
        feed.jump_to_latest();
    }
    world
        .entity_mut(body)
        .insert(ScrollPosition(Vec2::new(0.0, maximum)));
}

fn mount(world: &mut World, root: Entity, encounter: u64) -> Entity {
    let panel = world
        .spawn((
            Name::new("Combat History"),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(130.0),
                width: Val::Px(480.0),
                max_width: Val::Percent(46.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(
                world
                    .resource::<LabyrinthAppearance>()
                    .detail
                    .with_alpha(0.6),
            ),
            GlobalZIndex(40),
            bevy::ui::FocusPolicy::Block,
            Interaction::None,
            HistoryPanel,
            bevy_gamekit::ui::UiTooltipAvoid,
            UiRegionRole::ActivityFeed,
            HistoryEncounter(encounter),
            ChildOf(root),
        ))
        .id();
    let header = column(
        world,
        panel,
        "History Controls",
        Node {
            width: Val::Percent(100.0),
            column_gap: Val::Px(4.0),
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
    );
    let title = label(
        world,
        header,
        "History Title",
        "Combat log",
        UiTextRole::Metadata,
    );
    world.entity_mut(title).insert(Node {
        flex_grow: 1.0,
        ..default()
    });
    let latest = control(
        world,
        header,
        "History Latest",
        "Latest",
        Action::LatestLog,
        false,
    );
    world.entity_mut(latest).insert((
        HistoryLatest,
        UiSkinOverrides {
            background: Some(Color::NONE),
            border: Some(Color::NONE),
            ..default()
        },
    ));
    let hide = dock::glyph_control(
        world,
        header,
        "History Hide",
        "",
        "Hide combat log",
        "",
        crate::ui::glyphs::Glyph::Cancel,
        Action::HideLog,
    );
    world
        .entity_mut(hide)
        .remove::<bevy_gamekit::ui::UiContextHelp>();
    let body = world
        .spawn((
            Name::new("History Scroll"),
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            HistoryBody,
            HistoryReading::default(),
            UiFeedScroll::default(),
            ScrollPosition::default(),
            ChildOf(panel),
        ))
        .id();
    for leading in [true, false] {
        world.spawn((
            Name::new(if leading {
                "History Earlier Extent"
            } else {
                "History Later Extent"
            }),
            Node {
                width: Val::Px(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            HistorySpacer(leading),
            Pickable::IGNORE,
            ChildOf(body),
        ));
    }
    panel
}

/// The native renderer defaults to word-only wrapping. Custom history names may
/// be one long token, so this local card opts into character fallback before layout.
pub(crate) fn wrap_inspection(
    state: Res<UiTooltipState>,
    mut text: Query<(&Name, &mut TextLayout)>,
) {
    if !state
        .subjects()
        .first()
        .is_some_and(|subject| subject.0.starts_with("labyrinth/history/"))
    {
        return;
    }
    for (name, mut layout) in &mut text {
        if name.as_str() == "Tooltip Description"
            && layout.linebreak != bevy::text::LineBreak::WordOrCharacter
        {
            layout.linebreak = bevy::text::LineBreak::WordOrCharacter;
        }
    }
}

fn tooltip_subject(encounter: u64, id: u64) -> UiTooltipSubject {
    UiTooltipSubject(format!("labyrinth/history/{encounter}/{id}"))
}

/// Only mounted records and the bounded open inspection chain need content.
/// Pins survive row recycling and menu suspension; revocation removes their keys.
fn refresh_tooltips(world: &mut World, view: &LabyrinthView, valid: bool) {
    if !valid {
        return;
    }
    let prefix = format!("labyrinth/history/{}/", view.encounter);
    let mut subjects = world
        .query_filtered::<&UiTooltipSource, With<HistoryRow>>()
        .iter(world)
        .map(|source| source.0.clone())
        .collect::<Vec<_>>();
    subjects.extend(
        world
            .resource::<UiTooltipState>()
            .subjects()
            .iter()
            .cloned(),
    );
    subjects.sort();
    subjects.dedup();
    for subject in subjects {
        let Some(id) = subject
            .0
            .strip_prefix(&prefix)
            .and_then(|id| id.parse::<u64>().ok())
        else {
            continue;
        };
        let Some(event) = world.resource::<EncounterHistory>().page(id, 1).pop() else {
            continue;
        };
        let content = UiTooltipContent {
            title: "Combat log entry".into(),
            body: entry(view, &event).text,
            ..default()
        };
        world
            .resource_mut::<UiTooltipCatalog>()
            .0
            .insert(subject, content);
    }
}

pub(super) fn present(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    let Some(root) = world
        .query_filtered::<Entity, With<BattleRoot>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let existing = world
        .query_filtered::<Entity, With<HistoryPanel>>()
        .iter(world)
        .next();
    if existing.is_none() && !ui.log_visible {
        return;
    }
    let panel = existing.unwrap_or_else(|| mount(world, root, view.encounter));
    let Some(body) = world
        .query_filtered::<Entity, With<HistoryBody>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let fresh = world
        .get::<HistoryEncounter>(panel)
        .is_none_or(|encounter| encounter.0 != view.encounter);
    if fresh {
        world
            .entity_mut(panel)
            .insert(HistoryEncounter(view.encounter));
        world.entity_mut(body).insert((
            UiFeedScroll::default(),
            ScrollPosition::default(),
            HistoryReading::default(),
        ));
    }
    let concealed = world
        .resource::<crate::presentation::CombatDisclosure>()
        .has_unknown();
    let bounds = world.resource::<EncounterHistory>().bounds;
    let cache_encounter = world.resource::<EncounterHistory>().encounter;
    let valid = cache_encounter == view.encounter && !concealed;
    if let Some(mut feed) = world.get_mut::<UiFeedScroll>(body) {
        feed.revision = if valid {
            bounds.next.saturating_sub(1)
        } else {
            0
        };
    }
    let display = if ui.log_visible {
        Display::Flex
    } else {
        Display::None
    };
    let max_height = (world
        .get::<HistorySafeBottom>(root)
        .map_or(370.0, |bottom| bottom.0)
        - 138.0)
        .max(70.0);
    if let Some(mut node) = world.get_mut::<Node>(panel) {
        node.display = display;
        node.height = Val::Px(max_height.min(260.0));
        node.max_height = Val::Px(max_height);
    }
    // Keep the bounded hidden tree and feed state. Revocation still replaces its
    // text immediately, so a hidden panel cannot retain undisclosed summaries.
    if !ui.log_visible && !concealed {
        refresh_tooltips(world, view, valid);
        return;
    }
    let row_height = ROW_HEIGHT * world.resource::<ResolvedUiMetrics>().content_scale;
    let (height, old_maximum) =
        world
            .get::<ComputedNode>(body)
            .map_or((max_height - 60.0, 0.0), |node| {
                (
                    (node.size().y * node.inverse_scale_factor).max(1.0),
                    ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0),
                )
            });
    let mut offset = world
        .get::<ScrollPosition>(body)
        .map_or(0.0, |position| position.y);
    let following = world
        .get::<UiFeedScroll>(body)
        .is_none_or(|feed| feed.follows_latest())
        && offset >= old_maximum - 1.0;
    let count = if valid { bounds.next - bounds.first } else { 0 };
    let first = if valid { bounds.first } else { 0 };
    if let Some(reading) = world.get::<HistoryReading>(body) {
        if reading.row_height > 0.0 && reading.row_height != row_height && !following {
            offset = (reading.anchor.saturating_sub(first) as f32 + reading.fraction) * row_height;
            world
                .entity_mut(body)
                .insert(ScrollPosition(Vec2::new(0.0, offset)));
        }
    }
    let wanted_offset = if following {
        (count as f32 * row_height - height).max(0.0)
    } else {
        offset
    };
    let anchor_offset = (wanted_offset / row_height).floor() as u64;
    world.entity_mut(body).insert(HistoryReading {
        row_height,
        anchor: first.saturating_add(anchor_offset),
        fraction: (wanted_offset / row_height).fract(),
    });
    let from = first
        .saturating_add(anchor_offset.saturating_sub(OVERSCAN))
        .min(if valid { bounds.next } else { 0 });
    let limit = (((height / row_height).ceil() as usize) + 2 * OVERSCAN as usize)
        .min(MOUNT_LIMIT)
        .min(HISTORY_PAGE_EVENTS);
    let end = from
        .saturating_add(limit as u64)
        .min(if valid { bounds.next } else { 0 });
    let events = world
        .resource::<EncounterHistory>()
        .page(from, limit)
        .into_iter()
        .map(|event| (event.id, event))
        .collect::<BTreeMap<_, _>>();
    let mut requested = false;
    let mut wanted = (from..end)
        .map(|id| {
            events.get(&id).map_or_else(
                || {
                    if !requested {
                        world.write_message(LabyrinthIntent::HistoryPage {
                            encounter: view.encounter,
                            from: id,
                        });
                        requested = true;
                    }
                    Entry {
                        id,
                        text: "Loading history…".into(),
                        heading: false,
                    }
                },
                |event| entry(view, event),
            )
        })
        .collect::<Vec<_>>();
    if wanted.is_empty() {
        wanted.push(Entry {
            id: 0,
            text: if concealed {
                "Combat details are concealed."
            } else if valid && bounds.is_empty() {
                "No outcomes yet."
            } else {
                "Loading history…"
            }
            .into(),
            heading: false,
        });
    }
    let previous = world
        .query::<(Entity, &HistoryRow)>()
        .iter(world)
        .map(|(entity, row)| (entity, row.0.clone()))
        .collect::<Vec<_>>();
    let spacers = world
        .query::<(Entity, &HistorySpacer)>()
        .iter(world)
        .map(|(entity, spacer)| (entity, spacer.0))
        .collect::<Vec<_>>();
    let mut children = Vec::new();
    if let Some((entity, _)) = spacers.iter().find(|(_, leading)| *leading) {
        world.get_mut::<Node>(*entity).expect("spacer node").height =
            Val::Px(from.saturating_sub(first) as f32 * row_height);
        children.push(*entity);
    }
    for wanted in wanted {
        let item = if let Some((entity, _)) = previous.iter().find(|(_, old)| old.id == wanted.id) {
            *entity
        } else {
            let item = column(
                world,
                body,
                &format!("History Entry {}", wanted.id),
                Node::default(),
            );
            let text = label(world, item, "History Summary", "", UiTextRole::Metadata);
            world.entity_mut(text).insert((
                UiTextStyle {
                    base_size: Some(14.0),
                    ..default()
                },
                Node {
                    width: Val::Percent(100.0),
                    min_width: Val::Px(0.0),
                    ..default()
                },
            ));
            item
        };
        if wanted.id != 0 && events.contains_key(&wanted.id) {
            if world.get::<Button>(item).is_none() {
                world
                    .entity_mut(item)
                    .insert(bevy_gamekit::ui::button(format!(
                        "History Entry {}",
                        wanted.id
                    )));
            }
            let subject = tooltip_subject(view.encounter, wanted.id);
            world.entity_mut(item).insert((
                AccessibleLabel::new(wanted.text.clone()),
                bevy_gamekit::ui::UiFocusId::new("labyrinth", subject.0.clone()),
                UiTooltipSource(subject.clone()),
                UiTooltipOpen(subject),
            ));
        } else {
            world.entity_mut(item).remove::<(
                Button,
                bevy_gamekit::ui::UiAction,
                bevy::input_focus::tab_navigation::TabIndex,
                UiTooltipSource,
                UiTooltipOpen,
                bevy_gamekit::ui::UiContextHelp,
            )>();
        }
        let focused = world.resource::<bevy::input_focus::InputFocusVisible>().0
            && world.resource::<InputFocus>().get() == Some(item);
        let border = if focused {
            world.resource::<UiTheme>().accent
        } else {
            Color::NONE
        };
        world.entity_mut(item).insert(BorderColor::all(border));
        world.entity_mut(item).insert(Node {
            width: Val::Percent(100.0),
            height: Val::Px(row_height),
            min_height: Val::Px(row_height),
            max_height: Val::Px(row_height),
            flex_shrink: 0.0,
            align_items: AlignItems::Start,
            border: UiRect::all(Val::Px(1.0)),
            overflow: Overflow::clip(),
            ..default()
        });
        let text = world
            .get::<Children>(item)
            .and_then(|children| children.first())
            .copied()
            .expect("summary label");
        set_text(world, text, wanted.text.clone());
        let color = if wanted.heading {
            world.resource::<UiTheme>().accent
        } else {
            world.resource::<UiTheme>().text
        };
        world.entity_mut(text).insert(UiSkinOverrides {
            text: Some(color),
            ..default()
        });
        world.entity_mut(item).insert(HistoryRow(wanted));
        children.push(item);
    }
    if let Some((entity, _)) = spacers.iter().find(|(_, leading)| !*leading) {
        world.get_mut::<Node>(*entity).expect("spacer node").height = Val::Px(if valid {
            bounds.next.saturating_sub(end) as f32 * row_height
        } else {
            0.0
        });
        children.push(*entity);
    }
    for (entity, _) in previous {
        if !children.contains(&entity) {
            world.despawn(entity);
        }
    }
    if world
        .get::<Children>(body)
        .is_none_or(|current| current.to_vec() != children)
    {
        world.entity_mut(body).replace_children(&children);
    }
    refresh_tooltips(world, view, valid);
    let unread = world
        .get::<UiFeedScroll>(body)
        .filter(|feed| !feed.follows_latest())
        .map_or(0, UiFeedScroll::unread);
    let latest_label = if unread > 0 {
        format!("Latest · {unread} new")
    } else {
        "Latest".into()
    };
    let latest_controls = world
        .query_filtered::<Entity, With<HistoryLatest>>()
        .iter(world)
        .collect::<Vec<_>>();
    for latest in latest_controls {
        let labels = world
            .get::<Children>(latest)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        for label in labels {
            if world.get::<Text>(label).is_some() {
                set_text(world, label, latest_label.clone());
            }
        }
        world
            .entity_mut(latest)
            .insert(AccessibleLabel::new(latest_label.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_rows_keep_action_target_consequence_and_turn_context() {
        let view = LabyrinthView {
            combat: Some(
                labyrinth_rules::Combat::new(42, labyrinth_rules::DEFAULT_HERO_ROSTER)
                    .expect("combat")
                    .snapshot(),
            ),
            ..default()
        };
        let kinds = [
            CombatEventKind::Action {
                actor: ActorId(1),
                action: CombatAction::Rescue { ally: ActorId(2) },
            },
            CombatEventKind::Rescued {
                source: ActorId(1),
                actor: ActorId(2),
                hp: 5,
            },
            CombatEventKind::DeathSave {
                actor: ActorId(2),
                roll: Some(4),
                failures: 2,
            },
            CombatEventKind::CorpseRemoved {
                actor: ActorId(101),
                expired: true,
            },
            CombatEventKind::TurnStarted {
                actor: ActorId(2),
                turn_id: 42,
            },
            CombatEventKind::RoundStarted { round: 7 },
        ];
        let rows = kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                entry(
                    &view,
                    &PresentedEvent {
                        id: index as u64 + 1,
                        event: labyrinth_rules::CombatEvent {
                            id: index as u64 + 1,
                            kind,
                        },
                    },
                )
            })
            .collect::<Vec<_>>();
        assert!(rows
            .first()
            .expect("action")
            .text
            .contains("Gatekeeper · Rescue → Knifehand"));
        assert!(rows.get(1).expect("consequence").text.contains("5 HP"));
        assert!(rows
            .get(2)
            .expect("death save")
            .text
            .contains("2/3 failures"));
        assert!(rows.get(3).expect("corpse").text.contains("expired"));
        assert!(rows.get(4).expect("turn").text.contains("Turn 42"));
        assert!(rows.get(5).expect("round").text.contains("Round 7"));
    }
}

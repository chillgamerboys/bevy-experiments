//! Game-owned structured combat history over shared native feed scrolling.
use super::*;
use bevy_gamekit::ui::{UiFeedScroll, UiTooltipOpen};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    id: u64,
    title: String,
    details: Vec<String>,
    skill: Option<SkillId>,
}

fn entries(view: &LabyrinthView) -> Vec<Entry> {
    let token = |id: ActorId| {
        view.combat
            .as_ref()
            .and_then(|s| s.actor(id).map(|a| actors::token(s, a)))
            .unwrap_or_else(|| format!("#{}", id.0))
    };
    let name = |id: ActorId| {
        view.combat
            .as_ref()
            .and_then(|s| {
                s.actor(id)
                    .map(|a| format!("{} {}", actors::token(s, a), a.name()))
            })
            .unwrap_or_else(|| format!("Character {}", id.0))
    };
    let mut result: Vec<Entry> = Vec::new();
    for item in &view.events {
        let heading = match item.event.kind {
            CombatEventKind::Action { actor, action } => Some((
                format!(
                    "{} · {}",
                    name(actor),
                    inspection::choice_title(Some(match action {
                        CombatAction::Skill { skill, .. } => Choice::Skill(skill),
                        CombatAction::Reposition { .. } => Choice::Reposition,
                        CombatAction::Rescue { .. } => Choice::Rescue,
                        CombatAction::Defend => Choice::Defend,
                        CombatAction::Wait => Choice::Wait,
                    }))
                ),
                if let CombatAction::Skill { skill, .. } = action {
                    Some(skill)
                } else {
                    None
                },
            )),
            CombatEventKind::TurnStarted { actor, .. } => {
                Some((format!("{} · turn starts", name(actor)), None))
            }
            CombatEventKind::RoundStarted { round } => {
                Some((format!("Round {round} · initiative rolled"), None))
            }
            _ => None,
        };
        if let Some((title, skill)) = heading {
            result.push(Entry {
                id: item.id,
                title,
                details: Vec::new(),
                skill,
            });
        } else {
            // Bounded retention can begin midway through an action. Never
            // attach those orphan outcomes to an unrelated later action.
            if result.is_empty() {
                result.push(Entry {
                    id: item.id,
                    title: "Earlier action · partial history".into(),
                    details: Vec::new(),
                    skill: None,
                });
            }
            let detail = match item.event.kind {
                CombatEventKind::Damage {
                    source,
                    target,
                    amount,
                    kind,
                    ..
                } => format!(
                    "{} → {} −{amount} HP ({kind:?})",
                    token(source),
                    name(target)
                ),
                CombatEventKind::Healed {
                    source,
                    target,
                    amount,
                } => {
                    format!("{} → {} +{amount} HP", token(source), name(target))
                }
                CombatEventKind::Moved { actor, rank } => format!("{} → rank {rank}", name(actor)),
                _ => item.event.to_string(),
            };
            if let Some(entry) = result.last_mut() {
                entry.details.push(detail);
            }
        }
    }
    result
}

#[derive(Component)]
struct HistoryPanel;
#[derive(Component)]
struct HistoryBody;
#[derive(Component, PartialEq, Eq)]
struct HistoryRow(Entry, bool, bool);
#[derive(Component)]
struct HistoryMode(LogMode);
#[derive(Component)]
struct HistoryLatest;
#[derive(Component)]
struct HistoryEncounter(u64);

pub(crate) fn scroll(world: &mut World, direction: i8) {
    let Some(entity) = world
        .query_filtered::<Entity, With<HistoryBody>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let Some(node) = world.get::<ComputedNode>(entity) else {
        return;
    };
    let height = node.size().y * node.inverse_scale_factor;
    let maximum = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
    let current = world.get::<ScrollPosition>(entity).map_or(0.0, |p| p.y);
    let next = match direction {
        100.. => maximum,
        ..=-100 => 0.0,
        _ => (current + f32::from(direction) * height * 0.85).clamp(0.0, maximum),
    };
    world
        .entity_mut(entity)
        .insert(ScrollPosition(Vec2::new(0.0, next)));
}

pub(super) fn present(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    let full = ui.log_mode == LogMode::History;
    let previous_panel = world
        .query::<(Entity, &HistoryMode)>()
        .iter(world)
        .map(|(e, mode)| (e, mode.0))
        .next();
    if let Some((entity, mode)) = previous_panel {
        if mode != ui.log_mode || ui.log_mode == LogMode::Hidden {
            world.despawn(entity);
        }
    }
    if ui.log_mode == LogMode::Hidden {
        return;
    }
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
    let panel = existing.unwrap_or_else(|| {
        let panel = world
            .spawn((
                Name::new("Combat History"),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Px(130.0),
                    width: Val::Px(360.0),
                    max_width: Val::Percent(46.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(if full {
                    world.resource::<LabyrinthAppearance>().detail
                } else {
                    world
                        .resource::<LabyrinthAppearance>()
                        .detail
                        .with_alpha(0.6)
                }),
                GlobalZIndex(40),
                bevy::ui::FocusPolicy::Block,
                Interaction::None,
                HistoryPanel,
                bevy_gamekit::ui::UiTooltipAvoid,
                HistoryMode(ui.log_mode),
                UiRegionRole::ActivityFeed,
                HistoryEncounter(view.encounter),
                ChildOf(root),
            ))
            .id();
        let header = column(
            world,
            panel,
            "History Controls",
            Node {
                width: Val::Percent(100.0),
                column_gap: Val::Px(6.0),
                justify_content: if full {
                    JustifyContent::Start
                } else {
                    JustifyContent::End
                },
                flex_shrink: 0.0,
                ..default()
            },
        );
        if full {
            control(
                world,
                header,
                "History Toggle",
                "Compact",
                Action::SetLogMode(LogMode::Compact),
                false,
            );
        }
        if full {
            let latest = control(
                world,
                header,
                "History Latest",
                "Latest",
                Action::LatestLog,
                false,
            );
            world.entity_mut(latest).insert(HistoryLatest);
        }
        if full {
            control(
                world,
                header,
                "History Hide",
                "Hide",
                Action::SetLogMode(LogMode::Hidden),
                false,
            );
        } else {
            dock::glyph_control(
                world,
                header,
                "History Hide",
                "",
                "Hide combat log",
                "",
                crate::ui::glyphs::Glyph::Cancel,
                Action::SetLogMode(LogMode::Hidden),
            );
        }
        world.spawn((
            Name::new("History Scroll"),
            Node {
                width: Val::Percent(100.0),
                flex_grow: if full { 1.0 } else { 0.0 },
                flex_shrink: if full { 1.0 } else { 0.0 },
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                overflow: if full {
                    Overflow::scroll_y()
                } else {
                    Overflow::clip()
                },
                ..default()
            },
            HistoryBody,
            UiFeedScroll::default(),
            ScrollPosition::default(),
            ChildOf(panel),
        ));
        panel
    });
    let Some(body) = world
        .query_filtered::<Entity, With<HistoryBody>>()
        .iter(world)
        .next()
    else {
        return;
    };
    if world
        .get::<HistoryEncounter>(panel)
        .is_some_and(|e| e.0 != view.encounter)
    {
        world
            .entity_mut(panel)
            .insert(HistoryEncounter(view.encounter));
        world
            .entity_mut(body)
            .insert((UiFeedScroll::default(), ScrollPosition::default()));
        ui.expanded_log.clear();
    }
    let safe_bottom = world
        .get::<bevy_gamekit::ui::UiTooltipBounds>(root)
        .map_or(330.0, |b| b.0.max.y);
    let max_height = (safe_bottom - 138.0).max(70.0);
    if let Some(mut node) = world.get_mut::<Node>(panel) {
        let height = if full {
            Val::Px(max_height.min(360.0))
        } else {
            Val::Auto
        };
        if node.height != height {
            node.height = height;
        }
        if node.max_height != Val::Px(max_height) {
            node.max_height = Val::Px(max_height);
        }
    }
    let concealed = world
        .resource::<crate::presentation::CombatDisclosure>()
        .has_unknown();
    let all = if concealed { Vec::new() } else { entries(view) };
    ui.expanded_log
        .retain(|id| all.iter().any(|entry| entry.id == *id));
    let revision = view.events.last().map_or(0, |event| event.id);
    if let Some(mut feed) = world.get_mut::<UiFeedScroll>(body) {
        feed.revision = revision;
    }
    let unread = world
        .get::<UiFeedScroll>(body)
        .map_or(0, UiFeedScroll::unread);
    for entity in world
        .query_filtered::<Entity, With<HistoryLatest>>()
        .iter(world)
        .collect::<Vec<_>>()
    {
        let value = if unread > 0 {
            format!("Latest · {unread} new")
        } else {
            "Latest".to_owned()
        };
        if let Some(children) = world.get::<Children>(entity).map(|c| c.to_vec()) {
            for child in children {
                if world.get::<Text>(child).is_some() {
                    set_text(world, child, value.clone());
                }
            }
        }
    }
    let mut wanted = if full {
        all
    } else {
        all.into_iter()
            .rev()
            .filter_map(|entry| {
                entry.details.first().map(|detail| Entry {
                    id: entry.id,
                    title: detail.clone(),
                    details: Vec::new(),
                    skill: None,
                })
            })
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    };
    if wanted.is_empty() {
        wanted.push(Entry {
            id: 0,
            title: if concealed {
                "Combat details are concealed.".into()
            } else {
                view.log
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "No outcomes yet.".into())
            },
            details: Vec::new(),
            skill: None,
        });
    }
    let previous = world
        .query::<(Entity, &HistoryRow)>()
        .iter(world)
        .map(|(e, r)| (e, r.0.clone(), r.1, r.2))
        .collect::<Vec<_>>();
    let mut children = Vec::new();
    for entry in wanted {
        let expanded = full && ui.expanded_log.contains(&entry.id);
        if let Some((entity, _, _, _)) = previous.iter().find(|(_, old, open, old_full)| {
            *old == entry && *open == expanded && *old_full == full
        }) {
            children.push(*entity);
            continue;
        }
        let item = column(
            world,
            body,
            &format!("History Entry {}", entry.id),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                row_gap: Val::Px(4.0),
                ..default()
            },
        );
        if full && !entry.details.is_empty() {
            control(
                world,
                item,
                format!("History Expand {}", entry.id),
                format!("{} {}", if expanded { "−" } else { "+" }, entry.title),
                Action::ExpandLog(entry.id),
                false,
            );
        } else {
            label(
                world,
                item,
                "History Summary",
                &entry.title,
                UiTextRole::Body,
            );
        }
        for detail in entry
            .details
            .iter()
            .take(if expanded { usize::MAX } else { 1 })
        {
            label(
                world,
                item,
                "History Outcome",
                detail,
                UiTextRole::Supporting,
            );
        }
        if expanded {
            if let Some(skill) = entry.skill {
                let subject = tooltips::ability_subject(skill);
                if world
                    .resource::<bevy_gamekit::ui::UiTooltipCatalog>()
                    .0
                    .contains_key(&subject)
                {
                    let fonts = world.resource::<UiFonts>().clone();
                    world
                        .spawn((
                            bevy_gamekit::ui::button(format!("History Ability {}", entry.id)),
                            UiSkin::Control,
                            bevy_gamekit::ui::UiFocusId::new(
                                "labyrinth-history",
                                format!("ability/{}", entry.id),
                            ),
                            UiTooltipOpen(subject),
                            ChildOf(item),
                        ))
                        .with_child(bevy_gamekit::ui::text(
                            &fonts,
                            UiTextRole::Body,
                            "Ability details ›",
                        ));
                }
            }
        }
        world
            .entity_mut(item)
            .insert(HistoryRow(entry, expanded, full));
        children.push(item);
    }
    for (entity, _, _, _) in previous {
        if !children.contains(&entity) {
            world.despawn(entity);
        }
    }
    let current = world
        .get::<Children>(body)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    if current != children {
        world.entity_mut(body).replace_children(&children);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::PresentedEvent;
    #[test]
    fn grouping_uses_typed_boundaries_and_handles_retained_partial_actions() {
        let mut view = LabyrinthView::default();
        let kinds = [
            CombatEventKind::Damage {
                source: ActorId(1),
                target: ActorId(101),
                amount: 2,
                kind: DamageKind::Direct,
            },
            CombatEventKind::Action {
                actor: ActorId(1),
                action: CombatAction::Wait,
            },
            CombatEventKind::TurnStarted {
                actor: ActorId(2),
                turn_id: 2,
            },
        ];
        view.events = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| PresentedEvent {
                id: i as u64 + 20,
                event: labyrinth_rules::CombatEvent {
                    id: i as u64 + 1,
                    kind,
                },
            })
            .collect();
        let grouped = entries(&view);
        assert_eq!(grouped.len(), 3);
        assert!(grouped.first().expect("partial").title.contains("partial"));
        assert_eq!(entries(&view), grouped, "snapshot replay is idempotent");
    }
}

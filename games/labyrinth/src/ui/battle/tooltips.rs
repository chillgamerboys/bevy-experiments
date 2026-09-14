//! Labyrinth's disclosed explanations. Shared inspection never imports these rules.

use super::*;
use bevy_gamekit::ui::{
    UiTooltipCatalog, UiTooltipContent, UiTooltipLink, UiTooltipSource, UiTooltipSubject,
};
use labyrinth_rules::{Effect, StatusKind};

#[derive(Component)]
struct Skillbook;

#[derive(Component, PartialEq, Eq)]
struct BookContents(Vec<(UiTooltipSubject, UiTooltipContent)>);

fn subject(value: impl Into<String>) -> UiTooltipSubject {
    UiTooltipSubject(format!("labyrinth/{}", value.into()))
}

pub(super) fn ability_subject(
    encounter: u64,
    actor: ActorId,
    skill: &labyrinth_rules::catalog::ContentId,
) -> UiTooltipSubject {
    subject(format!(
        "encounter/{encounter}/actor/{}/skill/{skill}",
        actor.0
    ))
}

pub(crate) fn actor_subject(encounter: u64, actor: ActorId) -> UiTooltipSubject {
    subject(format!("encounter/{encounter}/actor/{}/details", actor.0))
}

pub(crate) fn effects_subject(encounter: u64, actor: ActorId) -> UiTooltipSubject {
    subject(format!("encounter/{encounter}/actor/{}/effects", actor.0))
}

fn condition_subject(kind: StatusKind) -> UiTooltipSubject {
    subject(format!("condition/{kind:?}"))
}

fn skill_content(skill: &labyrinth_rules::build::ResolvedSkill) -> UiTooltipContent {
    let definition = &skill.definition;
    let mut links = vec![UiTooltipLink {
        label: "Formation ranks".to_owned(),
        subject: subject("ranks"),
    }];
    for effect in &definition.effects {
        if let Effect::ApplyStatus(kind) = effect {
            links.push(UiTooltipLink {
                label: status_definition(*kind).name.to_owned(),
                subject: condition_subject(*kind),
            });
        }
    }
    let mut facts = vec![
        crate::presentation::effects_description(&definition.effects),
        format!("From  {}", rank_diagram(definition.source_ranks)),
        format!("Target {}", rank_diagram(definition.target_ranks)),
        definition.max_uses.map_or_else(
            || "Unlimited uses".to_owned(),
            |uses| format!("{uses} uses per encounter"),
        ),
    ];
    if definition.target_pattern == labyrinth_rules::catalog::TargetPattern::FrontPair {
        facts.push("Hits distinct occupants of front ranks 1–2 once each".to_owned());
    }
    for grant in &skill.grants {
        facts.push(format!(
            "{:?} grant · {} · {}",
            grant.kind, grant.provenance, grant.definition
        ));
    }
    for upgrade in &skill.upgrades {
        facts.push(format!(
            "Ability upgrade · {} · {}",
            upgrade.source.provenance, upgrade.source.definition
        ));
        for operation in &upgrade.upgrade.operations {
            use labyrinth_rules::catalog::UpgradeOperation;
            facts.push(match operation {
                UpgradeOperation::AddDamage { amount, .. } => format!("Effect power +{amount}"),
                UpgradeOperation::AppendEffect(effect) => {
                    crate::presentation::effects_description(std::slice::from_ref(effect))
                }
                UpgradeOperation::ExtendSourceRanks(mask) => {
                    format!("Added acting ranks {}", rank_diagram(*mask))
                }
                UpgradeOperation::ExtendTargetRanks(mask) => {
                    format!("Added target ranks {}", rank_diagram(*mask))
                }
            });
        }
    }
    UiTooltipContent {
        title: definition.name.clone(),
        facts,
        body: definition.description.clone(),
        links,
    }
}

fn rank_diagram(mask: u8) -> String {
    (1..=PARTY_SIZE)
        .map(|rank| {
            if mask & (1 << (rank - 1)) != 0 {
                rank.to_string()
            } else {
                "·".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}

pub(super) fn refresh(world: &mut World, view: &LabyrinthView, ui: &UiState) {
    let host = world
        .query_filtered::<Entity, With<BattleRoot>>()
        .iter(world)
        .next();
    if let Some(host) = host {
        let bottoms = world
            .query::<&ActorTile>()
            .iter(world)
            .filter_map(|tile| {
                let layout = world
                    .get::<crate::scene::SceneActorLayout>(tile.control)
                    .map_or(tile.control, |layout| layout.0);
                let node = world.get::<ComputedNode>(layout)?;
                let transform = world.get::<UiGlobalTransform>(layout)?;
                let actor = view.combat.as_ref()?.actor(tile.actor)?;
                let area = node.size() * node.inverse_scale_factor;
                let bottom = transform.translation.y * node.inverse_scale_factor + area.y * 0.46;
                // Reserve HP and commands, but allow help over the artwork.
                // Sparse formations fit tall actors; restricting help above
                // their heads can leave only enough height for its title.
                let history = crate::scene::actor_art_size(world, actor.kind, area)
                    .map_or(bottom, |art| bottom - art.y - 12.0);
                Some((bottom - 12.0, history))
            })
            .reduce(|(help, history), (other_help, other_history)| {
                (help.min(other_help), history.min(other_history))
            });
        if let Some((bottom, history_bottom)) = bottoms {
            let width = world
                .get::<ComputedNode>(host)
                .map_or(0.0, |node| node.size().x * node.inverse_scale_factor);
            if width > 0.0 && bottom > 100.0 {
                let bounds = bevy_gamekit::ui::UiTooltipBounds(Rect::from_corners(
                    Vec2::new(0.0, 60.0),
                    Vec2::new(width, bottom),
                ));
                if world.get::<bevy_gamekit::ui::UiTooltipBounds>(host) != Some(&bounds) {
                    world.entity_mut(host).insert(bounds);
                }
                let history = super::history::HistorySafeBottom(history_bottom);
                if history_bottom > 100.0
                    && world.get::<super::history::HistorySafeBottom>(host) != Some(&history)
                {
                    world.entity_mut(host).insert(history);
                }
            }
        }
    }
    let Some(snapshot) = view.combat.as_ref() else {
        return;
    };
    let projection = crate::presentation::BattlePresentation::new(
        snapshot,
        world.resource::<crate::presentation::CombatDisclosure>(),
    );
    let displayed = inspection::display_actor(view);
    let skills = displayed
        .and_then(|actor| projection.actor(actor.id))
        .and_then(|actor| actor.details.as_known())
        .map_or_else(Vec::new, |actor| actor.skills.clone());
    let mut entries = BTreeMap::new();
    for actor in &snapshot.actors {
        let Some(facts) = projection.actor(actor.id) else {
            continue;
        };
        let health = facts.health.as_known().map_or_else(
            || "HP unknown".into(),
            |hp| format!("{} / {} HP", hp.current, hp.maximum),
        );
        let speed = facts.details.as_known().map_or_else(
            || "Speed unknown".into(),
            |details| format!("Speed {}", details.speed),
        );
        let rank = snapshot.ranks(actor.id).map_or_else(
            || "Out of formation".into(),
            |ranks| {
                if ranks.start() == ranks.end() {
                    format!("Rank {}", ranks.start())
                } else {
                    format!("Ranks {}–{}", ranks.start(), ranks.end())
                }
            },
        );
        let owner = if view.local && actor.team() == Team::Heroes {
            "Local control"
        } else {
            view.players
                .iter()
                .find(|p| p.actors.contains(&actor.id))
                .map_or("Host AI", |p| p.name.as_str())
        };
        let mut rows = vec![format!("{health} · {rank}"), format!("{speed} · {owner}")];
        match actor.life {
            labyrinth_rules::LifeState::Dying { failures } => rows.push(format!(
                "Dying · {failures}/3 failed saves. Rescue before permanent death."
            )),
            labyrinth_rules::LifeState::Corpse { created_round, .. } => rows.push(format!(
                "Corpse · clears after round {}. Damage can clear it sooner; cannot rescue.",
                created_round.saturating_add(labyrinth_rules::CORPSE_ROUNDS)
            )),
            labyrinth_rules::LifeState::Removed => {
                rows.push("Permanently dead · remains cleared".into())
            }
            labyrinth_rules::LifeState::Alive => {}
        }
        if let Some(roll) = snapshot
            .initiative
            .iter()
            .find(|roll| roll.actor == actor.id)
        {
            let state = if snapshot.active_actor == Some(actor.id) {
                "Acting now"
            } else if roll.completed {
                "Completed this round"
            } else {
                "Waiting this round"
            };
            rows.push(if facts.details.as_known().is_some() {
                format!(
                    "{state} · rolled Speed {} + d8 {} = {}",
                    roll.speed, roll.roll, roll.total
                )
            } else {
                format!("{state} · initiative details unknown")
            });
        }
        let mut links = Vec::new();
        match facts.statuses.as_known() {
            Some(statuses) if statuses.is_empty() => rows.push("No active conditions".into()),
            Some(statuses) => {
                rows.push(format!("{} active conditions", statuses.len()));
                links.push(UiTooltipLink {
                    label: "Current conditions".into(),
                    subject: effects_subject(view.encounter, actor.id),
                });
            }
            None => rows.push("Status effects unknown".into()),
        }
        if ui.target == Some(actor.id) {
            if let Some(preview) = inspection::forecast_display(world, view, ui) {
                rows.push(format!("Selected action · {}", preview.summary));
            }
        }
        entries.insert(
            actor_subject(view.encounter, actor.id),
            UiTooltipContent {
                title: actors::title(snapshot, actor),
                facts: rows,
                links,
                ..default()
            },
        );
    }
    let anchors = world
        .query::<(Entity, &Action)>()
        .iter(world)
        .filter_map(|(entity, action)| match action {
            Action::Actor(actor) => Some((entity, actor_subject(view.encounter, *actor), false)),
            Action::InspectActor(actor) => {
                Some((entity, actor_subject(view.encounter, *actor), true))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for (entity, key, opens) in anchors {
        world
            .entity_mut(entity)
            .insert(UiTooltipSource(key.clone()));
        if opens {
            world
                .entity_mut(entity)
                .insert(bevy_gamekit::ui::UiTooltipOpen(key));
        }
    }
    entries.insert(subject("ranks"), UiTooltipContent {
        title: "Formation ranks".to_owned(), body: "Rank 1 is nearest the breach. Each side has six linear positions. Lit numbers show where an skill can be used and which target ranks it can reach. Character names and monster types identify combatants; the numbers at their feet show their current ranks.".to_owned(), ..default()
    });
    entries.insert(subject("boundaries"), UiTooltipContent {
        title: "Condition timing".to_owned(), body: "Conditions tick or expire on their declared boundary, not when you inspect them. Turn-start damage happens at the bearer's initiative slot, including while dying. Corpses never take turns: retained conditions trigger and count down at round end, before corpse expiry. Initiative is rolled again each round.".to_owned(), ..default()
    });
    for kind in [
        StatusKind::Bleed,
        StatusKind::Brace,
        StatusKind::Haste,
        StatusKind::Weakened,
    ] {
        let definition = status_definition(kind);
        entries.insert(
            condition_subject(kind),
            UiTooltipContent {
                title: definition.name.to_owned(),
                body: definition.description.to_owned(),
                links: vec![UiTooltipLink {
                    label: "Condition timing".to_owned(),
                    subject: subject("boundaries"),
                }],
                ..default()
            },
        );
    }
    let book = displayed.map_or_else(Vec::new, |actor| {
        skills
            .iter()
            .map(|skill| {
                (
                    ability_subject(view.encounter, actor.id, &skill.definition.id),
                    skill_content(skill),
                )
            })
            .collect::<Vec<_>>()
    });
    // Frozen cards are actor-scoped: two wielders can have different upgrades to
    // the same content ID. Revoking disclosure removes history and pinned cards too.
    for actor in &snapshot.actors {
        if let Some(details) = projection
            .actor(actor.id)
            .and_then(|facts| facts.details.as_known())
        {
            for skill in &details.skills {
                entries.insert(
                    ability_subject(view.encounter, actor.id, &skill.definition.id),
                    skill_content(skill),
                );
            }
        }
    }
    // Instance cards are viewer/encounter scoped and disappear when disclosure or
    // the condition changes. They are never sourced from hidden authority state.
    let badges = world
        .query::<(Entity, &StatusBadge)>()
        .iter(world)
        .map(|(entity, badge)| (entity, badge.actor))
        .collect::<Vec<_>>();
    for (entity, actor_id) in badges {
        let Some(actor) = projection.actor(actor_id) else {
            continue;
        };
        let Some(statuses) = actor
            .statuses
            .as_known()
            .filter(|statuses| !statuses.is_empty())
        else {
            continue;
        };
        let key = effects_subject(view.encounter, actor_id);
        let facts = statuses
            .iter()
            .map(|status| {
                crate::presentation::status_description(
                    status,
                    snapshot
                        .actor(actor_id)
                        .expect("projected actor exists")
                        .life,
                )
            })
            .collect();
        let links = statuses
            .iter()
            .map(|status| UiTooltipLink {
                label: status_definition(status.kind).name.to_owned(),
                subject: condition_subject(status.kind),
            })
            .collect();
        entries.insert(
            key.clone(),
            UiTooltipContent {
                title: "Current conditions".to_owned(),
                facts,
                links,
                ..default()
            },
        );
        world.entity_mut(entity).insert((
            UiTooltipSource(key.clone()),
            bevy_gamekit::ui::UiTooltipOpen(key.clone()),
        ));
    }
    world
        .resource_mut::<UiTooltipCatalog>()
        .0
        .retain(|key, _| !key.0.starts_with("labyrinth/"));
    world.resource_mut::<UiTooltipCatalog>().0.extend(entries);
    skillbook(world, ui.show_skillbook, book);
}

fn skillbook(world: &mut World, shown: bool, book: Vec<(UiTooltipSubject, UiTooltipContent)>) {
    let existing = world
        .query_filtered::<Entity, With<Skillbook>>()
        .iter(world)
        .next();
    if !shown {
        if let Some(entity) = existing {
            world.despawn(entity);
        }
        return;
    }
    if existing.is_some_and(|entity| {
        world
            .get::<BookContents>(entity)
            .is_some_and(|old| old.0 == book)
    }) {
        return;
    }
    if let Some(entity) = existing {
        world.despawn(entity);
    }
    let Some(root) = world
        .query_filtered::<Entity, With<BattleRoot>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let panel = column(
        world,
        root,
        "Skillbook",
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(64.0),
            right: Val::Px(16.0),
            width: Val::Px(370.0),
            max_width: Val::Percent(85.0),
            max_height: Val::Percent(70.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(14.0)),
            row_gap: Val::Px(8.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    );
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    world.entity_mut(panel).insert((
        Skillbook,
        BackgroundColor(appearance.detail),
        GlobalZIndex(55),
        Interaction::None,
        bevy::ui::FocusPolicy::Block,
        Pickable {
            should_block_lower: true,
            is_hoverable: true,
        },
    ));
    label(
        world,
        panel,
        "Skillbook Title",
        "Equipped skills",
        UiTextRole::Title,
    );
    control(
        world,
        panel,
        "Close Skillbook",
        "Close · K",
        Action::ToggleSkillbook,
        false,
    );
    for (key, content) in &book {
        let entity = world
            .spawn((
                bevy_gamekit::ui::button(format!("Read {}", content.title)),
                UiSkin::Control,
                bevy_gamekit::ui::UiControlMetrics::default(),
                bevy_gamekit::ui::UiFocusId::new("labyrinth-skillbook", key.0.clone()),
                UiTooltipSource(key.clone()),
                bevy_gamekit::ui::UiTooltipOpen(key.clone()),
                ChildOf(panel),
            ))
            .id();
        label(
            world,
            entity,
            "Skillbook Ability",
            &content.title,
            UiTextRole::Body,
        );
    }
    if book.is_empty() {
        label(
            world,
            panel,
            "Skillbook Empty",
            "No disclosed skills",
            UiTextRole::Body,
        );
    }
    world.entity_mut(panel).insert(BookContents(book));
}

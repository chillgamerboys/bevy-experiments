//! Labyrinth's disclosed explanations. Shared inspection never imports these rules.

use super::*;
use bevy_game_ui::{
    UiTooltipCatalog, UiTooltipContent, UiTooltipLink, UiTooltipSource, UiTooltipSubject,
};
use labyrinth_rules::{Effect, StatusKind};

#[derive(Component)]
struct Skillbook;

#[derive(Component, PartialEq, Eq)]
struct BookContents(Vec<(SkillId, UiTooltipContent)>);

fn subject(value: impl Into<String>) -> UiTooltipSubject {
    UiTooltipSubject(format!("labyrinth/{}", value.into()))
}

pub(super) fn ability_subject(skill: SkillId) -> UiTooltipSubject {
    subject(format!("ability/{skill:?}"))
}

fn condition_subject(kind: StatusKind) -> UiTooltipSubject {
    subject(format!("condition/{kind:?}"))
}

fn ability_content(skill: SkillId) -> UiTooltipContent {
    let definition = skill_definition(skill);
    let mut links = vec![UiTooltipLink {
        label: "Formation ranks".to_owned(),
        subject: subject("ranks"),
    }];
    for effect in definition.effects {
        if let Effect::ApplyStatus(kind) = effect {
            links.push(UiTooltipLink {
                label: status_definition(*kind).name.to_owned(),
                subject: condition_subject(*kind),
            });
        }
    }
    UiTooltipContent {
        title: definition.name.to_owned(),
        facts: vec![
            crate::presentation::base_description(&CombatAction::Skill {
                skill,
                target: ActorId(0),
            }),
            format!("From  {}", rank_diagram(definition.source_ranks)),
            format!("Target {}", rank_diagram(definition.target_ranks)),
        ],
        body: definition.description.to_owned(),
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
        let bottom = world
            .query::<&ActorTile>()
            .iter(world)
            .filter_map(|tile| {
                let node = world.get::<ComputedNode>(tile.control)?;
                let transform = world.get::<UiGlobalTransform>(tile.control)?;
                let actor = view.combat.as_ref()?.actor(tile.actor)?;
                let area = node.size() * node.inverse_scale_factor;
                let bottom = transform.translation.y * node.inverse_scale_factor + area.y * 0.46;
                // Even before artwork loads, never place help over HP or Confirm.
                Some(
                    crate::scene::actor_art_size(world, actor.kind, area)
                        .map_or(bottom, |art| bottom - art.y - 12.0),
                )
            })
            .reduce(f32::min);
        if let Some(bottom) = bottom {
            let width = world
                .get::<ComputedNode>(host)
                .map_or(0.0, |node| node.size().x * node.inverse_scale_factor);
            if width > 0.0 && bottom > 100.0 {
                let bounds = bevy_game_ui::UiTooltipBounds(Rect::from_corners(
                    Vec2::new(0.0, 60.0),
                    Vec2::new(width, bottom),
                ));
                if world.get::<bevy_game_ui::UiTooltipBounds>(host) != Some(&bounds) {
                    world.entity_mut(host).insert(bounds);
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
    let skills = inspection::display_actor(view)
        .and_then(|actor| projection.actor(actor.id))
        .and_then(|actor| actor.details.as_known())
        .map_or_else(Vec::new, |actor| actor.skills.clone());
    let mut entries = BTreeMap::new();
    entries.insert(subject("ranks"), UiTooltipContent {
        title: "Formation ranks".to_owned(), body: "Rank 1 is nearest the breach. Each side has six linear positions. Lit numbers show where an ability can be used and which target ranks it can reach. H1–H6 and E1–E6 identify actors, not their changing rank.".to_owned(), ..default()
    });
    entries.insert(subject("boundaries"), UiTooltipContent {
        title: "Condition timing".to_owned(), body: "Conditions tick or expire on their declared boundary, not when you inspect them. Turn-start damage happens before the actor chooses an action. Initiative is rolled again each round.".to_owned(), ..default()
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
    let book = skills
        .iter()
        .map(|skill| (*skill, ability_content(*skill)))
        .collect::<Vec<_>>();
    for (skill, content) in &book {
        entries.insert(ability_subject(*skill), content.clone());
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
        let key = subject(format!(
            "encounter/{}/actor/{}/effects",
            view.encounter, actor_id.0
        ));
        let facts = statuses
            .iter()
            .map(|status| {
                format!(
                    "{} · potency {} · {} boundaries left",
                    status_definition(status.kind).name,
                    status.potency,
                    status.remaining
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
        world.entity_mut(entity).insert(UiTooltipSource(key));
    }
    world
        .resource_mut::<UiTooltipCatalog>()
        .0
        .retain(|key, _| !key.0.starts_with("labyrinth/"));
    world.resource_mut::<UiTooltipCatalog>().0.extend(entries);
    skillbook(world, ui.show_skillbook, book);
}

fn skillbook(world: &mut World, shown: bool, book: Vec<(SkillId, UiTooltipContent)>) {
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
        "Equipped abilities",
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
    for (skill, content) in &book {
        let key = ability_subject(*skill);
        let entity = world
            .spawn((
                bevy_game_ui::button(format!("Read {}", content.title)),
                UiSkin::Control,
                bevy_game_ui::UiControlMetrics::default(),
                bevy_game_ui::UiFocusId::new("labyrinth-skillbook", format!("{skill:?}")),
                UiTooltipSource(key.clone()),
                bevy_game_ui::UiTooltipOpen(key),
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
            "No disclosed abilities",
            UiTextRole::Body,
        );
    }
    world.entity_mut(panel).insert(BookContents(book));
}

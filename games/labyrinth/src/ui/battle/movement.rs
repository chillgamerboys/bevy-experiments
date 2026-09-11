//! Non-interactive destination markers; live actor anchors never move for a preview.

use super::*;
use crate::presentation::{ForecastDisplay, ForecastOutcome, Knowledge};

#[derive(Component, Clone)]
struct FormationPreview {
    team: Team,
    heading: Entity,
    row: Entity,
    markers: BTreeMap<ActorId, Entity>,
}

pub(super) fn mount(world: &mut World, parent: Entity, team: Team, snapshot: &CombatSnapshot) {
    // Keep the live rank container exclusively actor-owned. The absolute preview
    // is its sibling, so formation reordering never needs to handle overlay nodes.
    let parent = world
        .get::<ChildOf>(parent)
        .expect("formation wrapper")
        .parent();
    let strip = column(
        world,
        parent,
        &format!("{team:?} Movement Preview"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            width: Val::Percent(100.0),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
    );
    world
        .entity_mut(strip)
        .insert((Pickable::IGNORE, ZIndex(2)));
    let heading = label(
        world,
        strip,
        &format!("{team:?} Movement Explanation"),
        "",
        UiTextRole::Supporting,
    );
    world
        .entity_mut(heading)
        .insert((Pickable::IGNORE, BackgroundColor(Color::NONE)));
    let row = column(
        world,
        strip,
        "Projected Rank Positions",
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(54.0),
            ..default()
        },
    );
    world.entity_mut(row).insert(Pickable::IGNORE);
    let mut markers = BTreeMap::new();
    for actor in snapshot.actors.iter().filter(|actor| actor.team() == team) {
        let marker = label(
            world,
            row,
            &format!("Actor {} Landing Marker", actor.id.0),
            "",
            UiTextRole::Body,
        );
        world.entity_mut(marker).insert((
            Node {
                position_type: PositionType::Absolute,
                height: Val::Percent(100.0),
                border: UiRect::bottom(Val::Px(2.0)),
                ..default()
            },
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ));
        markers.insert(actor.id, marker);
    }
    world.entity_mut(strip).insert(FormationPreview {
        team,
        heading,
        row,
        markers,
    });
}

pub(super) fn present(
    world: &mut World,
    view: &LabyrinthView,
    ui: &UiState,
    metrics: ResolvedUiMetrics,
) {
    let forecast = (!ui.menus.is_open() && !view.paused)
        .then(|| inspection::forecast_display(world, view, ui))
        .flatten();
    let strips = world
        .query::<(Entity, &FormationPreview)>()
        .iter(world)
        .map(|(entity, strip)| (entity, strip.clone()))
        .collect::<Vec<_>>();
    for (entity, strip) in strips {
        let visible =
            view.combat
                .as_ref()
                .zip(forecast.as_ref())
                .is_some_and(|(snapshot, forecast)| {
                    !forecast.uncertainty
                        && forecast.actors.iter().any(|change| {
                            snapshot
                                .actor(change.actor)
                                .is_some_and(|actor| actor.team() == strip.team)
                                && (change.position.is_some() || forecast.movement.is_some())
                        })
                });
        world.get_mut::<Node>(entity).expect("preview root").display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        if !visible {
            // Revoke cached text as well as pixels when disclosure/selection changes.
            set_text(world, strip.heading, String::new());
            for marker in strip.markers.values() {
                set_text(world, *marker, String::new());
                world.entity_mut(*marker).remove::<AccessibleLabel>();
            }
            continue;
        }
        if let Some((snapshot, forecast)) = view.combat.as_ref().zip(forecast.as_ref()) {
            update(world, &strip, snapshot, forecast, metrics);
        }
    }
}

fn rank_label(start: u8, footprint: u8) -> String {
    if footprint == 1 {
        start.to_string()
    } else {
        format!("{start}–{}", start + footprint - 1)
    }
}

fn update(
    world: &mut World,
    strip: &FormationPreview,
    snapshot: &CombatSnapshot,
    forecast: &ForecastDisplay,
    metrics: ResolvedUiMetrics,
) {
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let heading = forecast.movement.as_ref().map_or_else(
        || "After action · projected ranks".to_owned(),
        |movement| format!("After action · {movement}"),
    );
    set_text(world, strip.heading, heading);
    world
        .entity_mut(strip.heading)
        .insert(BackgroundColor(appearance.dock));
    world.get_mut::<Node>(strip.row).expect("rank row").height =
        Val::Px(54.0 * metrics.content_scale);
    let cleared = |id| {
        forecast.actors.iter().any(|change| {
            change.actor == id && change.outcome == Knowledge::Known(ForecastOutcome::CorpseCleared)
        })
    };
    let total: u8 = snapshot
        .formation(strip.team)
        .iter()
        .filter(|id| !cleared(**id))
        .filter_map(|id| snapshot.actor(*id))
        .map(|actor| actor.kind.footprint())
        .sum();
    for (id, marker) in &strip.markers {
        let Some(actor) = snapshot.actor(*id) else {
            continue;
        };
        let change = forecast.actors.iter().find(|change| change.actor == *id);
        let rank = snapshot.rank(*id).filter(|_| {
            !change.is_some_and(|change| {
                change.outcome == Knowledge::Known(ForecastOutcome::CorpseCleared)
            })
        });
        let Some(from) = rank else {
            world.get_mut::<Node>(*marker).expect("marker").display = Display::None;
            set_text(world, *marker, String::new());
            world.entity_mut(*marker).remove::<AccessibleLabel>();
            continue;
        };
        let to = change
            .and_then(|change| change.position)
            .map_or(from, |position| position.to);
        let width = actor.kind.footprint();
        let offset = if strip.team == Team::Heroes {
            total - (to + width - 1)
        } else {
            to - 1
        };
        let moved = from != to;
        let identity = actors::token(snapshot, actor);
        let ranks = if moved {
            format!("{} → {}", rank_label(from, width), rank_label(to, width))
        } else {
            rank_label(to, width)
        };
        set_text(world, *marker, format!("{identity}\n{ranks}"));
        world.entity_mut(*marker).insert((
            AccessibleLabel::new(format!(
                "After action: {identity} {}, ranks {} to {}",
                actor.name(),
                rank_label(from, width),
                rank_label(to, width)
            )),
            BackgroundColor(if moved {
                appearance.detail
            } else {
                appearance.dock.with_alpha(0.65)
            }),
            BorderColor::all(if moved {
                appearance.accent
            } else {
                appearance.line
            }),
            UiSkinOverrides {
                text: Some(if moved {
                    appearance.accent
                } else {
                    appearance.muted
                }),
                ..default()
            },
        ));
        let mut node = world.get_mut::<Node>(*marker).expect("marker");
        node.display = Display::Flex;
        node.left = Val::Percent(f32::from(offset) / f32::from(total) * 100.0);
        node.width = Val::Percent(f32::from(width) / f32::from(total) * 100.0);
        node.height = Val::Percent(100.0);
    }
}

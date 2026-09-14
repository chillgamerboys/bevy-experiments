//! A persistent draft preview, independent of catalog selection and scrolling.
use super::*;

#[derive(Component)]
enum PreviewText {
    Name,
    Stats,
    Equipment,
    Owner,
}
#[derive(Component)]
struct PreviewArt(Vec2);

pub(super) fn mount(
    world: &mut World,
    parent: Entity,
    editor: &ActorEditor,
    view: &LabyrinthView,
    compact: bool,
) {
    let width = if compact { 170. } else { 280. };
    let area = Vec2::new(width - 24., if compact { 150. } else { 260. });
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let panel = column(
        world,
        parent,
        "Character Preview",
        Node {
            width: Val::Px(width),
            flex_shrink: 0.,
            min_width: Val::Px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(14.),
            padding: UiRect::all(Val::Px(12.)),
            overflow: Overflow::clip(),
            ..default()
        },
    );
    world
        .entity_mut(panel)
        .insert(BackgroundColor(appearance.dock));
    let art = column(
        world,
        panel,
        "Character Sprite Area",
        Node {
            height: Val::Px(area.y),
            flex_shrink: 0.,
            align_items: AlignItems::End,
            justify_content: JustifyContent::Center,
            ..default()
        },
    );
    world.spawn((
        Name::new("Character Sprite"),
        ImageNode {
            color: Color::NONE,
            ..default()
        },
        Node::default(),
        PreviewArt(area),
        Pickable::IGNORE,
        ChildOf(art),
    ));
    for (key, kind, role) in [
        (
            "Character Preview Name",
            PreviewText::Name,
            UiTextRole::Title,
        ),
        (
            "Character Preview Stats",
            PreviewText::Stats,
            UiTextRole::Body,
        ),
        (
            "Character Preview Equipment",
            PreviewText::Equipment,
            UiTextRole::Body,
        ),
        (
            "Character Preview Owner",
            PreviewText::Owner,
            UiTextRole::Supporting,
        ),
    ] {
        let entity = label(world, panel, key, "", role);
        world.entity_mut(entity).insert((
            kind,
            Node {
                max_width: Val::Percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ));
    }
    update(world, editor, view);
}

pub(super) fn update(world: &mut World, editor: &ActorEditor, view: &LabyrinthView) {
    let weapon = editor
        .draft
        .actor
        .build
        .weapon
        .as_ref()
        .and_then(|id| view.catalog.as_ref()?.weapon(id))
        .map_or("Unarmed", |item| item.name.as_str());
    let owner: String = if details::rank_span(view, editor).0 == labyrinth_rules::Team::Enemies {
        "Enemy · host controlled".into()
    } else if view.local {
        "Local character".into()
    } else {
        view.company
            .iter()
            .find(|m| m.actor == editor.id)
            .and_then(|m| view.players.iter().find(|p| p.slot == m.owner))
            .map_or_else(
                || "Unassigned".into(),
                |p| format!("Controlled by {}", p.name),
            )
    };
    for (kind, mut text) in world.query::<(&PreviewText, &mut Text)>().iter_mut(world) {
        text.0 = match kind {
            PreviewText::Name => editor.name.clone(),
            PreviewText::Stats => format!(
                "{} maximum HP\nSpeed {} · {} rank spaces",
                editor.max_hp, editor.speed, editor.footprint
            ),
            PreviewText::Equipment => weapon.to_owned(),
            PreviewText::Owner => owner.clone(),
        };
    }
    let kind = editor.draft.actor.appearance;
    let image = world
        .get_resource::<crate::scene::SceneAppearance>()
        .and_then(|a| {
            let handle = a.actor_image(kind)?.clone();
            let image = world.get_resource::<Assets<Image>>()?.get(&handle)?;
            Some(ImageNode {
                rect: a.actor_rect(image.size(), kind),
                image: handle,
                ..default()
            })
        });
    let Some(image) = image else {
        return;
    };
    let targets = world
        .query::<(Entity, &PreviewArt)>()
        .iter(world)
        .map(|(entity, art)| (entity, art.0))
        .collect::<Vec<_>>();
    for (entity, area) in targets {
        let size = crate::scene::actor_art_size(world, kind, area).unwrap_or(area);
        world.entity_mut(entity).insert((
            image.clone(),
            Node {
                width: Val::Px(size.x),
                height: Val::Px(size.y),
                flex_shrink: 0.,
                ..default()
            },
        ));
    }
}

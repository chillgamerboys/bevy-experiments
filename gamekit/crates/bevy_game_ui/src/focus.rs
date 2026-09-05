//! Shared activation eligibility, scoped focus restoration, and modal navigation.

use super::*;

#[derive(Component)]
pub(crate) struct ManagedScrollArea;

pub(crate) fn prepare_scrolling(
    nodes: Query<
        (
            Entity,
            &Node,
            Has<bevy::ui_widgets::ScrollArea>,
            Has<ManagedScrollArea>,
        ),
        Changed<Node>,
    >,
    mut commands: Commands,
) {
    for (entity, node, has_area, managed) in &nodes {
        let scrolls =
            node.overflow.x == OverflowAxis::Scroll || node.overflow.y == OverflowAxis::Scroll;
        if scrolls && !has_area {
            commands
                .entity(entity)
                .insert((bevy::ui_widgets::ScrollArea, ManagedScrollArea));
        } else if !scrolls && managed {
            commands
                .entity(entity)
                .remove::<(bevy::ui_widgets::ScrollArea, ManagedScrollArea)>();
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct ModalFocusState(Vec<ModalFocusFrame>);

struct ModalFocusFrame {
    root: Entity,
    return_focus: Option<Entity>,
    return_focus_id: Option<UiFocusId>,
}

#[derive(Resource, Default)]
pub(crate) struct ScopedFocusMemory(Option<UiFocusId>);

pub(crate) fn remember_scoped_focus(
    focus: Res<InputFocus>,
    identities: Query<&UiFocusId>,
    mut memory: ResMut<ScopedFocusMemory>,
) {
    let Some(entity) = focus.get() else { return };
    let Ok(identity) = identities.get(entity) else {
        return;
    };
    memory.0 = Some(identity.clone());
}

/// Whether a control can receive activation now, including ancestors and modal scope.
#[must_use]
pub fn activation_eligible(world: &mut World, entity: Entity) -> bool {
    if !is_reachable(world, entity) {
        return false;
    }
    let topmost = {
        let mut query =
            world.query_filtered::<(Entity, Option<&GlobalZIndex>), With<UiModalScope>>();
        query
            .iter(world)
            .filter(|(entity, _)| is_reachable(world, *entity))
            .max_by_key(|(entity, z)| (z.map_or(0, |index| index.0), entity.to_bits()))
            .map(|(entity, _)| entity)
    };
    topmost.is_none_or(|root| is_descendant(world, entity, root))
}

pub(crate) fn emit_activations(world: &mut World) {
    let mut candidates = {
        let mut query = world
            .query_filtered::<(Entity, &Interaction), (Changed<Interaction>, With<UiAction>)>();
        query
            .iter(world)
            .filter_map(|(entity, interaction)| {
                (*interaction == Interaction::Pressed).then_some(entity)
            })
            .collect::<Vec<_>>()
    };
    if world
        .resource::<ButtonInput<KeyCode>>()
        .any_just_pressed([KeyCode::Enter, KeyCode::Space])
    {
        if let Some(entity) = world.resource::<InputFocus>().get() {
            if world.get::<UiAction>(entity).is_some() {
                candidates.push(entity);
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    for entity in candidates {
        if activation_eligible(world, entity) {
            world.write_message(UiActivated { entity });
        }
    }
}

fn editable_value(editable: &EditableText) -> String {
    let mut value = String::new();
    value.reserve(editable.value().into_iter().map(str::len).sum());
    for part in editable.value() {
        value.push_str(part);
    }
    value
}

pub(crate) fn emit_text_field_messages(
    keys: Res<ButtonInput<Key>>,
    focus: Res<InputFocus>,
    changed: Query<(Entity, &EditableText), (With<UiTextField>, Changed<EditableText>)>,
    fields: Query<&EditableText, With<UiTextField>>,
    mut changed_messages: MessageWriter<UiTextChanged>,
    mut submitted_messages: MessageWriter<UiTextSubmitted>,
) {
    for (entity, editable) in &changed {
        changed_messages.write(UiTextChanged {
            entity,
            value: editable_value(editable),
        });
    }
    if !keys.just_pressed(Key::Enter) {
        return;
    }
    let Some(entity) = focus.get() else { return };
    let Ok(editable) = fields.get(entity) else {
        return;
    };
    if !editable.is_composing() {
        submitted_messages.write(UiTextSubmitted {
            entity,
            value: editable_value(editable),
        });
    }
}

pub(crate) fn prepare_actions(
    added: Query<
        (
            Entity,
            Option<&Name>,
            Option<&TabIndex>,
            Option<&LogicalTabIndex>,
        ),
        Added<UiAction>,
    >,
    mut commands: Commands,
) {
    for (entity, name, tab_index, existing) in &added {
        let logical =
            existing.map_or_else(|| tab_index.map_or(0, |index| index.0), |index| index.0);
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((LogicalTabIndex(logical), TabIndex(logical)));
        if let Some(name) = name {
            entity_commands.insert(AccessibleLabel::new(name.as_str().to_owned()));
        }
    }
}

pub(crate) fn sync_action_reachability(world: &mut World) {
    let actions = {
        let mut query = world
            .query_filtered::<(Entity, &LogicalTabIndex), Or<(With<UiAction>, With<UiTextField>)>>(
            );
        query
            .iter(world)
            .map(|(entity, index)| (entity, index.0, world.get::<UiDisabled>(entity).is_some()))
            .collect::<Vec<_>>()
    };
    for (entity, logical_index, _disabled) in actions {
        let reachable = activation_eligible(world, entity);
        let next = TabIndex(if reachable { logical_index } else { -1 });
        if let Some(mut current) = world.get_mut::<TabIndex>(entity) {
            current.set_if_neq(next);
        }
    }
    let focused = world.resource::<InputFocus>().get();
    if let Some(entity) = focused.filter(|entity| !is_reachable(world, *entity)) {
        let identity = world.resource::<ScopedFocusMemory>().0.clone();
        let replacement = if world.get_entity(entity).is_err() {
            restore_focus_target(world, None, identity.as_ref())
        } else {
            None
        };
        let replacement = replacement.filter(|entity| activation_eligible(world, *entity));
        let mut focus = world.resource_mut::<InputFocus>();
        if let Some(entity) = replacement {
            focus.set(entity, FocusCause::Navigated);
        } else {
            focus.clear();
        }
    }
}

fn restore_focus_target(
    world: &mut World,
    entity: Option<Entity>,
    identity: Option<&UiFocusId>,
) -> Option<Entity> {
    if let Some(entity) = entity.filter(|entity| is_reachable(world, *entity)) {
        return Some(entity);
    }
    let wanted = identity?;
    let mut identities = world.query::<(Entity, &UiFocusId)>();
    let mut matches = identities.iter(world).filter_map(|(entity, identity)| {
        (identity == wanted && is_reachable(world, entity)).then_some(entity)
    });
    let target = matches.next()?;
    matches.next().is_none().then_some(target)
}

pub(crate) fn retain_modal_focus(world: &mut World) {
    let topmost = {
        let mut query =
            world.query_filtered::<(Entity, Option<&GlobalZIndex>), With<UiModalScope>>();
        query
            .iter(world)
            .filter(|(entity, _)| is_reachable(world, *entity))
            .max_by_key(|(entity, z)| (z.map_or(0, |index| index.0), entity.to_bits()))
            .map(|(entity, _)| entity)
    };
    let current_focus = world.resource::<InputFocus>().get();
    let mut frames = std::mem::take(&mut world.resource_mut::<ModalFocusState>().0);
    let previous = frames.last().map(|frame| frame.root);
    let mut target = current_focus;
    if previous != topmost {
        match topmost {
            Some(root) => {
                if let Some(index) = frames.iter().position(|frame| frame.root == root) {
                    // A nested modal closed: restore the control in the still-mounted parent.
                    if let Some(closed) = frames.get(index + 1) {
                        target = restore_focus_target(
                            world,
                            closed.return_focus,
                            closed.return_focus_id.as_ref(),
                        );
                    }
                    frames.truncate(index + 1);
                } else {
                    let mut frame = ModalFocusFrame {
                        root,
                        return_focus: current_focus
                            .filter(|entity| !is_descendant(world, *entity, root)),
                        return_focus_id: current_focus
                            .and_then(|entity| world.get::<UiFocusId>(entity))
                            .cloned()
                            .or_else(|| world.resource::<ScopedFocusMemory>().0.clone()),
                    };
                    if !frames.is_empty()
                        && frames.iter().all(|frame| !is_reachable(world, frame.root))
                    {
                        // A game rebuilt its entire view: preserve the original outer return identity.
                        if let Some(original) = frames.first() {
                            frame.return_focus = original.return_focus;
                            frame.return_focus_id = original.return_focus_id.clone();
                        }
                        frames.clear();
                    }
                    frames.push(frame);
                }
            }
            None => {
                if let Some(original) = frames.first() {
                    target = restore_focus_target(
                        world,
                        original.return_focus,
                        original.return_focus_id.as_ref(),
                    );
                }
                frames.clear();
            }
        }
    }
    if let Some(root) = topmost {
        if target.is_none_or(|entity| {
            !is_descendant(world, entity, root) || !is_reachable(world, entity)
        }) {
            target = first_reachable_action(world, root);
        }
    }
    world.resource_mut::<ModalFocusState>().0 = frames;
    if target != current_focus {
        let mut focus = world.resource_mut::<InputFocus>();
        if let Some(target) = target {
            focus.set(target, FocusCause::Navigated);
        } else {
            focus.clear();
        }
    }
}

fn first_reachable_action(world: &World, root: Entity) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if (world.get::<UiAction>(entity).is_some() || world.get::<UiTextField>(entity).is_some())
            && is_reachable(world, entity)
        {
            return Some(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    None
}

fn is_descendant(world: &World, mut entity: Entity, root: Entity) -> bool {
    loop {
        if entity == root {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

fn is_reachable(world: &World, mut entity: Entity) -> bool {
    if world.get_entity(entity).is_err() {
        return false;
    }
    loop {
        if world.get::<UiDisabled>(entity).is_some()
            || world.get::<InteractionDisabled>(entity).is_some()
            || world
                .get::<Visibility>(entity)
                .is_some_and(|visibility| *visibility == Visibility::Hidden)
            || world
                .get::<Node>(entity)
                .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return true;
        };
        entity = parent.parent();
    }
}

pub(crate) fn scroll_focused_into_view(focus: Res<InputFocus>, mut commands: Commands) {
    if focus.is_changed() {
        if let Some(entity) = focus.get() {
            commands.trigger(ScrollIntoView { entity });
        }
    }
}

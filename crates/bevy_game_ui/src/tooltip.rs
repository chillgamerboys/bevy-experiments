//! Opt-in, local-only inspection. Content and disclosure remain game-owned.

#[cfg(test)]
mod tests;
mod view;

use std::{collections::BTreeMap, time::Duration};

use bevy::{ecs::message::MessageCursor, input_focus::InputFocus, prelude::*};

use crate::{UiActivated, UiContextHelp, UiContextHelpState, UiContextHelpSystems};

/// Allows contextual inspection of a disabled control, without enabling it.
#[derive(Component)]
pub struct UiInspectable;

/// Opaque, game-owned content identity. Never use a translated title as identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiTooltipSubject(pub String);

/// Associates an anchor with durable content, independent of its entity lifetime.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiTooltipSource(pub UiTooltipSubject);

/// Attach to an existing action to open inspection on activation, without a
/// game-specific action translator. Do not attach to gameplay ability buttons.
#[derive(Component, Debug, Clone)]
pub struct UiTooltipOpen(pub UiTooltipSubject);

#[derive(Component)]
pub(crate) struct TooltipOrigin(pub Entity);

/// A disclosed term that opens another card; the shared UI knows no domain enums.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiTooltipLink {
    /// Visible, accessible link label.
    pub label: String,
    /// Key in [`UiTooltipCatalog`]. Missing keys are not rendered as links.
    pub subject: UiTooltipSubject,
}

/// Structured, already-disclosed explanation, reusable in a game's skillbook.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiTooltipContent {
    /// Heading, rendered once.
    pub title: String,
    /// Compact factual rows such as base power or a game's formation diagram.
    pub facts: Vec<String>,
    /// Optional prose, never permanently attached to the action bar.
    pub body: String,
    /// Explicit related terms. Cycles truncate the chain instead of growing it.
    pub links: Vec<UiTooltipLink>,
}

/// Game-maintained disclosed content. Remove a key when it becomes unavailable.
/// Open cards refresh on changes; removing a parent also closes its descendants.
#[derive(Resource, Default)]
pub struct UiTooltipCatalog(pub BTreeMap<UiTooltipSubject, UiTooltipContent>);

/// Put on one screen root to opt into the default native tooltip renderer.
/// The host must fill its UI render target. Despawning it dismisses its chain.
#[derive(Component)]
pub struct UiTooltipHost;

/// Optional safe rectangle in host-local logical pixels, for example above a
/// game's health strips. Placement and scroll height are confined to this area.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiTooltipBounds(pub Rect);

/// Timing is driven by Bevy's real-time clock, not wall-clock sleeps or simulation time.
#[derive(Resource, Debug, Clone)]
pub struct UiTooltipSettings {
    /// Quiet hover/focus dwell before a preview appears.
    pub show_delay: Duration,
    /// Time allowed to cross gaps between source, card, and child cards.
    pub leave_grace: Duration,
    /// Bounded number of cards, including the root (clamped to 1–8).
    pub max_depth: usize,
    /// Optional local inspect shortcut; games may rebind it or use requests only.
    pub inspect_key: Option<KeyCode>,
    /// Optional deepest-first dismiss shortcut.
    pub dismiss_key: Option<KeyCode>,
}

impl Default for UiTooltipSettings {
    fn default() -> Self {
        Self {
            show_delay: Duration::from_millis(350),
            leave_grace: Duration::from_millis(450),
            max_depth: 4,
            inspect_key: Some(KeyCode::KeyT),
            dismiss_key: Some(KeyCode::Escape),
        }
    }
}

/// Local inspection commands; these never represent gameplay activation.
#[derive(Message, Debug, Clone)]
pub enum UiTooltipRequest {
    /// Open a known subject immediately (for example, from a skillbook).
    Open(UiTooltipSubject),
    /// Keep the current chain open independently of pointer position.
    Pin,
    /// Close the deepest card first.
    Back,
    /// Close the complete chain.
    Dismiss,
}

/// Read-only lifecycle state for game input arbitration and custom renderers.
#[derive(Resource, Default)]
pub struct UiTooltipState {
    pub(crate) chain: Vec<UiTooltipSubject>,
    pub(crate) transient: Option<(UiTooltipSubject, UiTooltipContent)>,
    pub(crate) anchor: Option<Entity>,
    pub(crate) pinned: bool,
    pub(crate) keyboard: bool,
    pub(crate) consumed: bool,
    candidate: Option<UiTooltipSubject>,
    dwell: Duration,
    away: Duration,
    return_focus: Option<Entity>,
    return_identity: Option<crate::UiFocusId>,
    suppressed: Option<UiTooltipSubject>,
    host: Option<Entity>,
}

impl UiTooltipState {
    /// Current root-to-leaf path. Subjects contain no authority or credentials.
    #[must_use]
    pub fn subjects(&self) -> &[UiTooltipSubject] {
        &self.chain
    }
    /// Whether explicit pinning is keeping this chain alive.
    #[must_use]
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }
    /// Games should skip their shortcuts when this is true. Escape is consumed
    /// for one frame even when it just closed the final card.
    #[must_use]
    pub fn captures_keyboard(&self) -> bool {
        self.keyboard || self.consumed
    }

    fn dismiss(&mut self) {
        self.chain.clear();
        self.pinned = false;
        self.keyboard = false;
        self.away = Duration::ZERO;
    }

    fn follow(&mut self, depth: usize, subject: UiTooltipSubject, maximum: usize) {
        self.chain.truncate(depth.saturating_add(1));
        if let Some(existing) = self.chain.iter().position(|key| key == &subject) {
            self.chain.truncate(existing + 1);
        } else if self.chain.len() < maximum.clamp(1, 8) {
            self.chain.push(subject);
        }
    }

    fn hover(
        &mut self,
        candidate: Option<UiTooltipSubject>,
        over_card: bool,
        delta: Duration,
        settings: &UiTooltipSettings,
    ) {
        if candidate != self.candidate {
            self.candidate = candidate.clone();
            self.dwell = Duration::ZERO;
            self.suppressed = None;
        }
        if self.pinned || self.keyboard || over_card {
            self.away = Duration::ZERO;
            return;
        }
        if let Some(subject) = candidate {
            self.away = Duration::ZERO;
            self.dwell = self.dwell.saturating_add(delta);
            if self.dwell >= settings.show_delay
                && self.suppressed.as_ref() != Some(&subject)
                && self.chain.first() != Some(&subject)
            {
                self.chain = vec![subject];
            }
        } else {
            self.away = self.away.saturating_add(delta);
            if self.away >= settings.leave_grace {
                self.dismiss();
            }
        }
    }
}

/// Ordering seam: games refresh disclosed catalogs before Resolve and skip local
/// shortcuts after it when [`UiTooltipState::captures_keyboard`] is true.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiTooltipSystems {
    /// Resolve lifecycle and consume local inspection input, after context selection.
    Resolve,
    /// Reconcile native cards after game presentation has updated its content.
    Render,
}

/// Adds delayed previews, sticky reading, explicit pinning, linked cards, and a
/// replaceable native renderer. Add alongside GameUiPlugin. `T` inspects/pins;
/// Escape closes deepest-first. Pointer previews never steal keyboard focus.
pub struct GameUiTooltipPlugin;

impl Plugin for GameUiTooltipPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::GameUiContextHelpPlugin>() {
            app.add_plugins(crate::GameUiContextHelpPlugin);
        }
        if !app.is_plugin_added::<crate::GameUiSkinPlugin>() {
            app.add_plugins(crate::GameUiSkinPlugin);
        }
        app.init_resource::<UiTooltipSettings>()
            .init_resource::<UiTooltipCatalog>()
            .init_resource::<UiTooltipState>()
            .init_resource::<view::TooltipView>()
            .add_message::<UiTooltipRequest>()
            .configure_sets(
                Update,
                UiTooltipSystems::Resolve.after(UiContextHelpSystems::Resolve),
            )
            .add_systems(
                Update,
                refresh_sources.before(UiContextHelpSystems::Resolve),
            )
            .add_systems(Update, resolve.in_set(UiTooltipSystems::Resolve))
            .add_systems(
                PostUpdate,
                view::render
                    .in_set(UiTooltipSystems::Render)
                    .before(bevy::ui::UiSystems::Prepare),
            )
            .add_systems(
                PostUpdate,
                view::place
                    .after(bevy::ui::UiSystems::PostLayout)
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
    }
}

fn refresh_sources(world: &mut World) {
    let sources = world
        .query::<(Entity, &UiTooltipSource)>()
        .iter(world)
        .map(|(entity, source)| (entity, source.0.clone()))
        .collect::<Vec<_>>();
    for (entity, subject) in sources {
        let content = world
            .resource::<UiTooltipCatalog>()
            .0
            .get(&subject)
            .cloned();
        if let Some(content) = content {
            let help = UiContextHelp {
                title: content.title,
                body: content.body,
            };
            if world.get::<UiContextHelp>(entity) != Some(&help) {
                world.entity_mut(entity).insert(help);
            }
        } else {
            world.entity_mut(entity).remove::<UiContextHelp>();
        }
    }
}

fn content(
    world: &World,
    state: &UiTooltipState,
    subject: &UiTooltipSubject,
) -> Option<UiTooltipContent> {
    world
        .resource::<UiTooltipCatalog>()
        .0
        .get(subject)
        .cloned()
        .or_else(|| {
            state
                .transient
                .as_ref()
                .filter(|(key, _)| key == subject)
                .and_then(|(_, value)| {
                    if let Some(anchor) = state
                        .anchor
                        .filter(|entity| subject.0 == format!("native-anchor:{}", entity.to_bits()))
                    {
                        let help = world.get::<UiContextHelp>(anchor)?;
                        Some(UiTooltipContent {
                            title: help.title.clone(),
                            body: help.body.clone(),
                            ..default()
                        })
                    } else {
                        Some(value.clone())
                    }
                })
        })
}

fn resolve(
    world: &mut World,
    mut requests: Local<MessageCursor<UiTooltipRequest>>,
    mut activations: Local<MessageCursor<UiActivated>>,
) {
    let commands = requests
        .read(world.resource::<Messages<UiTooltipRequest>>())
        .cloned()
        .collect::<Vec<_>>();
    let activated = activations
        .read(world.resource::<Messages<UiActivated>>())
        .map(|message| message.entity)
        .collect::<Vec<_>>();
    let clicked = activated
        .iter()
        .filter_map(|entity| world.get::<view::TooltipAction>(*entity).cloned())
        .collect::<Vec<_>>();
    let opened = activated
        .iter()
        .filter_map(|entity| {
            world
                .get::<UiTooltipOpen>(*entity)
                .map(|open| (*entity, open.0.clone()))
        })
        .collect::<Vec<_>>();
    let delta = world
        .get_resource::<Time<Real>>()
        .map_or(Duration::ZERO, Time::delta);
    let settings = world.resource::<UiTooltipSettings>().clone();
    let help = world.resource::<UiContextHelpState>().clone();
    let candidate = help.entity.and_then(|entity| {
        if world.get::<view::TooltipAction>(entity).is_some() {
            return None;
        }
        if let Some(source) = world.get::<UiTooltipSource>(entity) {
            return Some((source.0.clone(), None, entity));
        }
        help.content.map(|help| {
            (
                UiTooltipSubject(format!("native-anchor:{}", entity.to_bits())),
                Some(UiTooltipContent {
                    title: help.title,
                    body: help.body,
                    ..default()
                }),
                entity,
            )
        })
    });
    let over_card = world
        .query_filtered::<&Interaction, With<view::TooltipSurface>>()
        .iter(world)
        .any(|interaction| *interaction != Interaction::None)
        || world
            .query::<(&Interaction, &view::TooltipAction)>()
            .iter(world)
            .any(|(interaction, _)| *interaction != Interaction::None);
    let host = world
        .query_filtered::<Entity, With<UiTooltipHost>>()
        .iter(world)
        .min();
    let modal = world
        .query_filtered::<Entity, With<crate::UiModalScope>>()
        .iter(world)
        .collect::<Vec<_>>()
        .into_iter()
        .any(|entity| crate::inspection_eligible(world, entity));
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let escape = settings
        .dismiss_key
        .is_some_and(|key| keys.just_pressed(key));
    let inspect = settings
        .inspect_key
        .is_some_and(|key| keys.just_pressed(key));
    let editing = world
        .resource::<InputFocus>()
        .get()
        .is_some_and(|entity| world.get::<crate::UiTextField>(entity).is_some());
    let page = if keys.just_pressed(KeyCode::PageDown) {
        1
    } else if keys.just_pressed(KeyCode::PageUp) {
        -1
    } else if keys.just_pressed(KeyCode::End) {
        100
    } else if keys.just_pressed(KeyCode::Home) {
        -100
    } else {
        0
    };
    world.resource_scope(|world, mut state: Mut<UiTooltipState>| {
        let was_keyboard = state.keyboard;
        let host_changed = state.host.is_some() && state.host != host;
        state.host = host;
        state.consumed = false;
        if page != 0 && !editing && !state.chain.is_empty() {
            view::scroll(world, page);
            state.consumed = true;
        }
        if let Some((subject, Some(value), _)) = &candidate {
            if !state.pinned || state.chain.first() == Some(subject) {
                state.transient = Some((subject.clone(), value.clone()));
            }
        }
        let key = candidate
            .as_ref()
            .map(|(key, _, _)| key.clone())
            .filter(|key| content(world, &state, key).is_some());
        state.hover(key.clone(), over_card, delta, &settings);
        if let Some((key, _, anchor)) = &candidate {
            if state.chain.first() == Some(key) && !state.pinned {
                state.anchor = Some(*anchor);
            }
        }
        for (entity, subject) in opened {
            if content(world, &state, &subject).is_some() {
                state.chain = vec![subject];
                state.anchor = Some(entity);
                state.pinned = true;
            }
        }
        for command in commands {
            match command {
                UiTooltipRequest::Open(subject) if content(world, &state, &subject).is_some() => {
                    state.chain = vec![subject];
                    state.pinned = true;
                }
                UiTooltipRequest::Pin => state.pinned = !state.chain.is_empty(),
                UiTooltipRequest::Back => {
                    state.chain.pop();
                }
                UiTooltipRequest::Dismiss => state.dismiss(),
                UiTooltipRequest::Open(_) => {}
            }
        }
        for action in clicked {
            match action {
                view::TooltipAction::Pin => state.pinned = !state.pinned,
                view::TooltipAction::Close(depth) => {
                    state.chain.truncate(depth);
                    state.suppressed = state.candidate.clone();
                }
                view::TooltipAction::Link(depth, subject) => {
                    if content(world, &state, &subject).is_some() {
                        state.follow(depth, subject, settings.max_depth);
                    }
                }
            }
        }
        if inspect && !editing {
            // A preview may still belong to the previous source during dwell.
            // Explicit inspection reads the new source immediately, but never
            // replaces a deliberately pinned or nested reading session.
            if state.chain.is_empty()
                || (state.chain.len() == 1 && !state.pinned && !state.keyboard && !over_card)
            {
                if let Some(key) = key {
                    state.chain = vec![key];
                    state.anchor = candidate.as_ref().map(|(_, _, entity)| *entity);
                }
            }
            if !state.chain.is_empty() {
                state.pinned = true;
                state.keyboard = true;
                state.consumed = true;
                if !was_keyboard {
                    state.return_focus = world.resource::<InputFocus>().get();
                    state.return_identity = state
                        .return_focus
                        .and_then(|entity| world.get::<crate::UiFocusId>(entity))
                        .cloned();
                }
            }
        }
        if escape && !state.chain.is_empty() && !editing {
            state.chain.pop();
            state.consumed = true;
            state.suppressed = state.candidate.clone();
        }
        // The catalog is the disclosure boundary, not the source entity's lifetime.
        if let Some(invalid) = state
            .chain
            .iter()
            .position(|key| content(world, &state, key).is_none())
        {
            state.chain.truncate(invalid);
        }
        if let Some((subject, _)) = &state.transient {
            if state.chain.first() == Some(subject)
                && state.anchor.is_none_or(|entity| {
                    world.get::<UiContextHelp>(entity).is_none()
                        || !crate::inspection_eligible(world, entity)
                })
            {
                state.dismiss();
            }
        }
        let blocked = modal
            && state
                .anchor
                .is_none_or(|entity| !crate::inspection_eligible(world, entity));
        if blocked || host.is_none() || host_changed {
            state.dismiss();
        }
        if state.chain.is_empty() {
            state.pinned = false;
            state.keyboard = false;
        }
        if was_keyboard && !state.keyboard {
            let target = state
                .return_focus
                .filter(|entity| crate::activation_eligible(world, *entity))
                .or_else(|| {
                    state.return_identity.as_ref().and_then(|identity| {
                        let matches = world
                            .query::<(Entity, &crate::UiFocusId)>()
                            .iter(world)
                            .filter(|(_, key)| *key == identity)
                            .map(|(entity, _)| entity)
                            .collect::<Vec<_>>();
                        (matches.len() == 1)
                            .then(|| matches.first().copied())
                            .flatten()
                            .filter(|entity| crate::activation_eligible(world, *entity))
                    })
                });
            if let Some(entity) = target {
                world
                    .resource_mut::<InputFocus>()
                    .set(entity, bevy::input_focus::FocusCause::Navigated);
            } else {
                world.resource_mut::<InputFocus>().clear();
            }
        }
    });
}

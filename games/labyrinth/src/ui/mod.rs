//! Game-owned native presentation: immutable views in, typed intent out.

mod appearance;
mod battle;
mod glyphs;
mod shell;
#[cfg(test)]
mod tests;

pub use appearance::LabyrinthAppearance;
use bevy::ecs::message::MessageCursor;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy_gamekit::ui::{
    GameUiSkinPlugin, GameUiSystems, GameUiTooltipPlugin, ResolvedUiMetrics, UiActivated, UiFonts,
    UiInsets, UiMotionPreference, UiScaleMode, UiScalePreference, UiSkin, UiSkinOverrides, UiSpace,
    UiSpacing, UiTextChanged, UiTextField, UiTextRole, UiTheme,
};
use labyrinth_rules::{ActorId, CombatAction, HeroClass, SkillId, PARTY_SIZE};

use crate::view::{HostSettings, LabyrinthIntent, LabyrinthView, SecretText, ViewMode};

/// Installs Labyrinth's own forms, battlefield, inspection, and presentation state.
pub struct LabyrinthUiPlugin;

/// Scheduling seams for the application projection and intent consumer.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabyrinthUiSystems {
    /// Translate native input to local selection and application intent.
    Input,
    /// Project the most recent immutable application view.
    Present,
}

/// Initial presentation preferences supplied by the composition root.
#[derive(Resource, Debug, Clone)]
pub struct LabyrinthUiConfig {
    /// Seed used by the Local and Host forms.
    pub seed: u64,
}

impl Default for LabyrinthUiConfig {
    fn default() -> Self {
        Self { seed: 42 }
    }
}

impl Plugin for LabyrinthUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameUiSkinPlugin,
            GameUiTooltipPlugin,
            bevy_gamekit::ui::GameUiFeedPlugin,
            crate::scene::LabyrinthScenePlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.025, 0.034, 0.038)))
        .init_resource::<UiState>()
        .init_resource::<LabyrinthAppearance>()
        .init_resource::<crate::presentation::CombatDisclosure>()
        .init_resource::<LabyrinthUiConfig>()
        .init_resource::<LabyrinthView>()
        .add_message::<LabyrinthIntent>()
        .insert_resource(UiTheme {
            background: Color::srgb(0.025, 0.035, 0.045),
            panel: Color::srgb(0.055, 0.075, 0.085),
            card: Color::srgb(0.085, 0.11, 0.12),
            control: Color::srgb(0.10, 0.14, 0.15),
            control_hovered: Color::srgb(0.15, 0.23, 0.23),
            control_pressed: Color::srgb(0.21, 0.31, 0.29),
            accent: Color::srgb(0.89, 0.75, 0.43),
            text: Color::srgb(0.94, 0.93, 0.86),
            muted_text: Color::srgb(0.75, 0.81, 0.79),
            edge: Color::srgb(0.28, 0.35, 0.34),
            body_size: 18.0,
            supporting_size: 18.0,
            title_size: 26.0,
            display_size: 48.0,
            ..UiTheme::default()
        })
        .configure_sets(
            Update,
            (LabyrinthUiSystems::Input, LabyrinthUiSystems::Present)
                .chain()
                .after(GameUiSystems::EmitActivations)
                .after(bevy_gamekit::ui::UiTooltipSystems::Resolve),
        )
        .add_systems(
            Update,
            (collect_text, collect_actions, keyboard_shortcuts)
                .chain()
                .in_set(LabyrinthUiSystems::Input),
        )
        .add_systems(Startup, load_default_font)
        .add_systems(
            Update,
            appearance::apply.before(GameUiSystems::EmitActivations),
        )
        .add_systems(Update, glyphs::refresh.after(LabyrinthUiSystems::Present))
        .configure_sets(
            Update,
            LabyrinthUiSystems::Present.after(bevy_gamekit::ui::UiContextHelpSystems::Resolve),
        )
        .add_systems(Update, present.in_set(LabyrinthUiSystems::Present));
    }
}

fn load_default_font(mut assets: ResMut<Assets<Font>>, mut fonts: ResMut<UiFonts>) {
    // Embedded with its OFL license, so launching from either workspace or crate works.
    // Adopters may supply a different font before startup without changing layout.
    if fonts.body != Handle::default() || fonts.heading != Handle::default() {
        return;
    }
    let font = Font::from_bytes(include_bytes!("assets/AlegreyaSans-Regular.ttf").to_vec());
    let handle = assets.add(font);
    fonts.body = handle.clone();
    fonts.heading = handle;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Form {
    #[default]
    Menu,
    Multiplayer,
    Host,
    Direct,
    Browser,
    Password,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuPage {
    Game,
    Settings,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Skill(SkillId),
    Reposition,
    Rescue,
    Defend,
    Wait,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum LogMode {
    #[default]
    Hidden,
    Compact,
    History,
}

#[derive(Resource, Default)]
struct UiState {
    form: Form,
    selected: Option<Choice>,
    target: Option<ActorId>,
    decision: Option<(u64, ActorId)>,
    encounter: Option<u64>,
    menus: bevy_gamekit::ui::UiMenuStack<MenuPage>,
    log_mode: LogMode,
    expanded_log: std::collections::BTreeSet<u64>,
    show_skillbook: bool,
    session_name: String,
    address: String,
    port: String,
    password: SecretText,
    code: SecretText,
    lan: bool,
    tailnet: bool,
    selected_session: Option<bevy_gamekit::session::SessionId>,
    local_notice: Option<String>,
    shell_key: Option<String>,
    overlay_key: Option<String>,
}

#[derive(Component, Debug, Clone)]
enum Action {
    Form(Form),
    StartLocal,
    Host,
    JoinCode,
    Browse,
    JoinDiscovered,
    Session(bevy_gamekit::session::SessionId),
    Reconnect,
    Leave,
    ConfirmLeave,
    GameMenu,
    ToggleLan,
    ToggleTailnet,
    Ready(bool),
    Hero(HeroClass),
    Start,
    Rematch,
    Copy(usize),
    Reissue(usize),
    Actor(ActorId),
    InspectActor(ActorId),
    Choice(Choice),
    SkillSlot(usize),
    Status(ActorId, u64),
    Confirm,
    Cancel,
    Settings,
    ReducedMotion,
    Scale,
    ToggleLog,
    ExpandLog(u64),
    LatestLog,
    SetLogMode(LogMode),
    ToggleSkillbook,
    ScrollDetails(i8),
}

#[derive(Component, Debug, Clone, Copy)]
enum Field {
    Name,
    Address,
    Port,
    Password,
    Code,
}

#[derive(Component)]
struct ShellRoot;
#[derive(Component)]
struct OverlayRoot;

fn collect_text(
    mut changed: MessageReader<UiTextChanged>,
    fields: Query<&Field>,
    mut ui: ResMut<UiState>,
) {
    for change in changed.read() {
        match fields.get(change.entity) {
            Ok(Field::Name) => ui.session_name.clone_from(&change.value),
            Ok(Field::Address) => ui.address.clone_from(&change.value),
            Ok(Field::Port) => ui.port.clone_from(&change.value),
            Ok(Field::Password) => ui.password = SecretText(change.value.clone()),
            Ok(Field::Code) => ui.code = SecretText(change.value.clone()),
            Err(_) => {}
        }
    }
}

fn collect_actions(world: &mut World, mut cursor: Local<MessageCursor<UiActivated>>) {
    let actions = {
        cursor
            .read(world.resource::<Messages<UiActivated>>())
            .map(|message| message.entity)
            .collect::<Vec<_>>()
    };
    for entity in actions {
        if let Some(action) = world.get::<Action>(entity).cloned() {
            apply_action(world, action);
        }
    }
}

fn keyboard_shortcuts(world: &mut World) {
    if world
        .resource::<bevy_gamekit::ui::UiTooltipState>()
        .captures_keyboard()
    {
        return;
    }
    let focus = world.resource::<InputFocus>().get();
    if focus.is_some_and(|entity| world.get::<UiTextField>(entity).is_some()) {
        return;
    }
    let keys = world.resource::<ButtonInput<KeyCode>>();
    if keys.just_pressed(KeyCode::KeyK) && !world.resource::<UiState>().menus.is_open() {
        apply_action(world, Action::ToggleSkillbook);
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        apply_action(world, Action::Cancel);
        return;
    }
    if world.resource::<UiState>().menus.is_open() || world.resource::<LabyrinthView>().paused {
        return;
    }
    let ui = world.resource::<UiState>();
    let details_open = ui.show_skillbook;
    {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        let page = if keys.just_pressed(KeyCode::PageUp) {
            -1
        } else if keys.just_pressed(KeyCode::PageDown) {
            1
        } else if keys.just_pressed(KeyCode::Home) {
            -100
        } else if keys.just_pressed(KeyCode::End) {
            100
        } else {
            0
        };
        if page != 0 {
            apply_action(world, Action::ScrollDetails(page));
            return;
        }
    }
    if details_open {
        return;
    }
    let shortcut = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ]
    .iter()
    .position(|key| keys.just_pressed(*key));
    if let Some(index) = shortcut {
        apply_action(world, Action::SkillSlot(index));
    }
}

fn apply_action(world: &mut World, action: Action) {
    let view = world.resource::<LabyrinthView>().clone();
    let seed = world.resource::<LabyrinthUiConfig>().seed;
    world.resource_scope(|world, mut ui: Mut<UiState>| {
        let intent =
            match action {
                Action::Form(form) => {
                    if view.mode == ViewMode::Menu
                        && form == Form::Menu
                        && matches!(ui.form, Form::Host | Form::Direct | Form::Password)
                    {
                        // Closing a form also cancels its asynchronous host/join attempt.
                        world.write_message(LabyrinthIntent::Leave);
                    }
                    if matches!(ui.form, Form::Browser | Form::Password) {
                        world.write_message(LabyrinthIntent::StopBrowsing);
                    }
                    ui.password = SecretText::default();
                    ui.code = SecretText::default();
                    ui.form = form;
                    ui.local_notice = None;
                    if form == Form::Browser {
                        Some(LabyrinthIntent::Browse {
                            tailnet: ui.tailnet,
                        })
                    } else {
                        None
                    }
                }
                Action::StartLocal => Some(LabyrinthIntent::StartLocal(seed)),
                Action::Host => {
                    let port = if ui.port.is_empty() {
                        Ok(7777)
                    } else {
                        ui.port.parse::<u16>()
                    };
                    match port {
                        Ok(port) if port > 0 => Some(LabyrinthIntent::Host(HostSettings {
                            name: if ui.session_name.trim().is_empty() {
                                "The Lantern Company".to_owned()
                            } else {
                                ui.session_name.clone()
                            },
                            password: std::mem::take(&mut ui.password),
                            address: ui.address.clone(),
                            port,
                            lan: ui.lan,
                            tailnet: ui.tailnet,
                            seed,
                        })),
                        _ => {
                            ui.local_notice =
                                Some("Game port must be between 1 and 65535.".to_owned());
                            None
                        }
                    }
                }
                Action::JoinCode => Some(LabyrinthIntent::JoinCode(std::mem::take(&mut ui.code))),
                Action::Browse => Some(LabyrinthIntent::Browse {
                    tailnet: ui.tailnet,
                }),
                Action::Session(session) => {
                    ui.selected_session = Some(session);
                    ui.form = Form::Password;
                    None
                }
                Action::JoinDiscovered => {
                    ui.selected_session
                        .map(|session| LabyrinthIntent::JoinDiscovered {
                            session,
                            password: std::mem::take(&mut ui.password),
                        })
                }
                Action::Reconnect => Some(LabyrinthIntent::Reconnect),
                Action::Leave => {
                    ui.menus.open(MenuPage::Leave);
                    None
                }
                Action::GameMenu => {
                    ui.menus.open(MenuPage::Game);
                    None
                }
                Action::ConfirmLeave => {
                    ui.menus.close();
                    ui.form = Form::Menu;
                    Some(LabyrinthIntent::Leave)
                }
                Action::ToggleLan => {
                    ui.lan = !ui.lan;
                    None
                }
                Action::ToggleTailnet => {
                    ui.tailnet = !ui.tailnet;
                    if ui.form == Form::Browser {
                        Some(LabyrinthIntent::Browse {
                            tailnet: ui.tailnet,
                        })
                    } else {
                        None
                    }
                }
                Action::Ready(ready) => Some(LabyrinthIntent::Ready(ready)),
                Action::Hero(hero) => Some(LabyrinthIntent::SelectHero(hero)),
                Action::Start => Some(LabyrinthIntent::StartEncounter),
                Action::Rematch => Some(LabyrinthIntent::Rematch),
                Action::Copy(index) => Some(LabyrinthIntent::CopyInvite(index)),
                Action::Reissue(index) => Some(LabyrinthIntent::ReissueInvite(index)),
                Action::Actor(actor) => {
                    ui.target = Some(actor);
                    None
                }
                Action::InspectActor(actor) => {
                    world.write_message(bevy_gamekit::ui::UiTooltipRequest::Open(
                        battle::actor_subject(view.encounter, actor),
                    ));
                    None
                }
                Action::Status(actor, _status) => {
                    world.write_message(bevy_gamekit::ui::UiTooltipRequest::Open(
                        battle::effects_subject(view.encounter, actor),
                    ));
                    None
                }
                Action::Choice(choice) => {
                    ui.selected = Some(choice);
                    None
                }
                Action::SkillSlot(index) => {
                    if battle::skills_disclosed(
                        &view,
                        world.resource::<crate::presentation::CombatDisclosure>(),
                    ) {
                        battle::select_skill_slot(&view, &mut ui, index);
                    }
                    None
                }
                Action::Confirm => {
                    battle::selected_action(&view, &ui)
                        .ok()
                        .map(|(actor, action)| LabyrinthIntent::Combat {
                            actor,
                            action,
                            encounter: view.encounter,
                            decision: view.combat.as_ref().map_or(0, |snapshot| snapshot.turn_id),
                        })
                }
                Action::ToggleSkillbook => {
                    ui.show_skillbook = !ui.show_skillbook;
                    None
                }
                Action::Cancel => {
                    if ui.show_skillbook && !ui.menus.is_open() {
                        ui.show_skillbook = false;
                        return;
                    }
                    let cancel_attempt = !ui.menus.is_open()
                        && ui.selected.is_none()
                        && view.mode == ViewMode::Menu
                        && matches!(ui.form, Form::Host | Form::Direct | Form::Password);
                    let stop_browser = !ui.menus.is_open()
                        && view.mode == ViewMode::Menu
                        && matches!(ui.form, Form::Browser | Form::Password);
                    if ui.menus.is_open() {
                        ui.menus.back();
                    } else if ui.log_mode != LogMode::Hidden {
                        ui.log_mode = LogMode::Hidden;
                    } else if ui.selected.is_some() {
                        ui.selected = None;
                        ui.target = None;
                    } else if view.mode == ViewMode::Menu {
                        ui.form = Form::Menu;
                        ui.password = SecretText::default();
                        ui.code = SecretText::default();
                    } else {
                        ui.menus.open(MenuPage::Game);
                    }
                    if cancel_attempt {
                        Some(LabyrinthIntent::Leave)
                    } else {
                        stop_browser.then_some(LabyrinthIntent::StopBrowsing)
                    }
                }
                Action::Settings => {
                    ui.menus.open(MenuPage::Settings);
                    None
                }
                Action::ReducedMotion => {
                    let mut preference = world.resource_mut::<UiMotionPreference>();
                    preference.reduced = !preference.reduced;
                    None
                }
                Action::Scale => {
                    let mut scale = world.resource_mut::<UiScalePreference>();
                    scale.0 = if scale.0 == UiScaleMode::Percent200 {
                        UiScaleMode::Auto
                    } else {
                        UiScaleMode::Percent200
                    };
                    None
                }
                Action::ToggleLog => {
                    ui.log_mode = if ui.log_mode == LogMode::History {
                        LogMode::Hidden
                    } else {
                        LogMode::History
                    };
                    None
                }
                Action::ExpandLog(id) => {
                    if !ui.expanded_log.remove(&id) {
                        ui.expanded_log.insert(id);
                    }
                    None
                }
                Action::LatestLog => {
                    let mut query = world.query::<&mut bevy_gamekit::ui::UiFeedScroll>();
                    for mut feed in query.iter_mut(world) {
                        feed.jump_to_latest();
                    }
                    None
                }
                Action::SetLogMode(mode) => {
                    ui.log_mode = mode;
                    if let Some(entity) = world.query::<(Entity, &Action)>().iter(world).find_map(
                        |(entity, action)| matches!(action, Action::ToggleLog).then_some(entity),
                    ) {
                        world
                            .resource_mut::<InputFocus>()
                            .set(entity, bevy::input_focus::FocusCause::Navigated);
                    }
                    None
                }
                Action::ScrollDetails(direction) => {
                    if ui.log_mode == LogMode::History {
                        battle::scroll_history(world, direction);
                    }
                    None
                }
            };
        if let Some(intent) = intent {
            if matches!(
                intent,
                LabyrinthIntent::Host(_)
                    | LabyrinthIntent::JoinCode(_)
                    | LabyrinthIntent::JoinDiscovered { .. }
            ) {
                clear_native_secret_fields(world);
            }
            world.write_message(intent);
        }
    });
}

fn clear_native_secret_fields(world: &mut World) {
    let entities = world
        .query::<(Entity, &Field)>()
        .iter(world)
        .filter(|(_, field)| matches!(field, Field::Password | Field::Code))
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    for entity in entities {
        if let Some(mut editable) = world.get_mut::<bevy::text::EditableText>(entity) {
            editable.clear();
        }
    }
}

fn present(world: &mut World) {
    let view = world.resource::<LabyrinthView>().clone();
    let metrics = *world.resource::<ResolvedUiMetrics>();
    world.resource_scope(|world, mut ui: Mut<UiState>| {
        if view.mode == ViewMode::Combat && view.combat.is_some() {
            despawn_marked::<ShellRoot>(world);
            ui.shell_key = None;
            battle::present(world, &view, &mut ui, metrics);
        } else {
            battle::clear(world);
            ui.selected = None;
            ui.target = None;
            ui.decision = None;
            ui.encounter = None;
            shell::present(world, &view, &mut ui, metrics);
        }
        shell::overlays(world, &view, &mut ui);
    });
}

fn despawn_marked<T: Component>(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, With<T>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.despawn(entity);
    }
}

fn label(
    world: &mut World,
    parent: Entity,
    name: &str,
    value: impl Into<String>,
    role: UiTextRole,
) -> Entity {
    let fonts = world.resource::<UiFonts>();
    let bundle = bevy_gamekit::ui::text(fonts, role, value);
    world
        .spawn((
            Name::new(name.to_owned()),
            bundle,
            UiSkin::Text,
            ChildOf(parent),
        ))
        .id()
}

fn control(
    world: &mut World,
    parent: Entity,
    key: impl Into<String>,
    value: impl Into<String>,
    action: Action,
    disabled: bool,
) -> Entity {
    let key = key.into();
    let value = value.into();
    let entity = world
        .spawn((
            bevy_gamekit::ui::button(key.clone()),
            UiSkin::Control,
            UiSpacing {
                padding: Some(UiInsets::axes(UiSpace::Units(1.0), UiSpace::Units(0.5))),
                ..default()
            },
            bevy_gamekit::ui::UiFocusId::new("labyrinth", key),
            action,
            ChildOf(parent),
        ))
        .id();
    world
        .entity_mut(entity)
        .insert(AccessibleLabel::new(value.clone()));
    if disabled {
        world
            .entity_mut(entity)
            .insert(bevy_gamekit::ui::UiDisabled);
    }
    label(world, entity, "Control Label", value, UiTextRole::Body);
    entity
}

fn column(world: &mut World, parent: Entity, name: &str, node: Node) -> Entity {
    world
        .spawn((Name::new(name.to_owned()), node, ChildOf(parent)))
        .id()
}

fn set_text(world: &mut World, entity: Entity, value: String) {
    let parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
    if let Some(mut text) = world.get_mut::<Text>(entity) {
        if text.0 != value {
            text.0.clone_from(&value);
            // A dynamic skill/actor label must not leave its accessible name stale.
            if let Some(parent) = parent {
                if world.get::<bevy_gamekit::ui::UiAction>(parent).is_some() {
                    world.entity_mut(parent).insert(AccessibleLabel::new(value));
                }
            }
        }
    }
}

fn set_disabled(world: &mut World, entity: Entity, disabled: bool) {
    if disabled && world.get::<bevy_gamekit::ui::UiDisabled>(entity).is_none() {
        world
            .entity_mut(entity)
            .insert(bevy_gamekit::ui::UiDisabled);
    } else if !disabled && world.get::<bevy_gamekit::ui::UiDisabled>(entity).is_some() {
        world
            .entity_mut(entity)
            .remove::<bevy_gamekit::ui::UiDisabled>();
    }
}

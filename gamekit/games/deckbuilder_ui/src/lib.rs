//! A deliberately small game-owned adopter of the reusable Gamekit crates.

use bevy::prelude::*;
use bevy_game_turns::TurnOrder;
use bevy_game_ui::{
    button, card, modal, panel, region, screen_root, text, GameUiSystems, ResolvedUiMetrics,
    UiActivated, UiDisabled, UiFonts, UiRegionRole, UiTextRole, UiViewportClass,
};

/// Installs the deckbuilder's local model, action translation, and presentation.
pub struct DeckbuilderPlugin;

impl Plugin for DeckbuilderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeckbuilderModel>()
            .init_resource::<UiDirty>()
            .add_systems(Startup, render_if_dirty)
            .add_systems(
                Update,
                (
                    handle_activations.after(GameUiSystems::EmitActivations),
                    mark_responsive_change,
                    render_if_dirty,
                )
                    .chain(),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Participant {
    Player,
    Opponent,
}

impl Participant {
    const fn label(self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::Opponent => "Opponent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Match,
}

#[derive(Debug, Clone)]
struct CardState {
    id: u8,
    title: &'static str,
    rules: &'static str,
    cost: u8,
    played: bool,
}

#[derive(Resource, Debug)]
struct DeckbuilderModel {
    screen: Screen,
    turns: TurnOrder<Participant>,
    energy: u8,
    cards: Vec<CardState>,
    selected: Option<u8>,
    paused: bool,
    activity: Vec<String>,
}

impl Default for DeckbuilderModel {
    fn default() -> Self {
        Self {
            screen: Screen::Menu,
            turns: TurnOrder::new([Participant::Player, Participant::Opponent])
                .expect("the fixed demo roster is non-empty and unique"),
            energy: 3,
            cards: vec![
                CardState {
                    id: 1,
                    title: "Spark",
                    rules: "Deal 1 damage.",
                    cost: 1,
                    played: false,
                },
                CardState {
                    id: 2,
                    title: "Ward",
                    rules: "Gain 2 armor.",
                    cost: 2,
                    played: false,
                },
                CardState {
                    id: 3,
                    title: "Comet",
                    rules: "Deal 5 damage.",
                    cost: 5,
                    played: false,
                },
            ],
            selected: None,
            paused: false,
            activity: vec!["Waiting in the tavern.".to_owned()],
        }
    }
}

#[derive(Resource, Debug)]
struct UiDirty(bool);

impl Default for UiDirty {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum DeckbuilderAction {
    Start,
    SelectCard(u8),
    PlaySelected,
    EndTurn,
    Pause,
    Resume,
    ReturnToMenu,
}

#[derive(Component)]
struct DeckbuilderUiRoot;

#[derive(Debug)]
enum DeckbuilderView {
    Menu,
    Match(MatchView),
}

#[derive(Debug)]
struct MatchView {
    turn: String,
    round: u64,
    energy: u8,
    cards: Vec<CardView>,
    can_play: bool,
    activity: Vec<String>,
    paused: bool,
}

#[derive(Debug)]
struct CardView {
    id: u8,
    title: &'static str,
    rules: &'static str,
    cost: u8,
    selected: bool,
    disabled: bool,
}

fn project(model: &DeckbuilderModel) -> DeckbuilderView {
    if model.screen == Screen::Menu {
        return DeckbuilderView::Menu;
    }
    let player_turn = model.turns.current() == Some(&Participant::Player);
    let cards = model
        .cards
        .iter()
        .map(|card| CardView {
            id: card.id,
            title: card.title,
            rules: card.rules,
            cost: card.cost,
            selected: model.selected == Some(card.id),
            disabled: card.played || card.cost > model.energy || !player_turn,
        })
        .collect::<Vec<_>>();
    let can_play = model.selected.is_some_and(|selected| {
        cards
            .iter()
            .any(|card| card.id == selected && !card.disabled)
    });
    DeckbuilderView::Match(MatchView {
        turn: model
            .turns
            .current()
            .map_or_else(|| "Finished".to_owned(), |turn| turn.label().to_owned()),
        round: model.turns.round(),
        energy: model.energy,
        cards,
        can_play,
        activity: model.activity.clone(),
        paused: model.paused,
    })
}

fn handle_activations(
    mut activations: MessageReader<UiActivated>,
    actions: Query<&DeckbuilderAction>,
    mut model: ResMut<DeckbuilderModel>,
    mut dirty: ResMut<UiDirty>,
) {
    for activation in activations.read() {
        let Ok(action) = actions.get(activation.entity) else {
            continue;
        };
        match *action {
            DeckbuilderAction::Start => {
                model.screen = Screen::Match;
                model.activity = vec!["The duel begins.".to_owned()];
            }
            DeckbuilderAction::SelectCard(id) => {
                let selectable = model.cards.iter().any(|card| {
                    card.id == id
                        && !card.played
                        && card.cost <= model.energy
                        && model.turns.current() == Some(&Participant::Player)
                });
                if selectable {
                    model.selected = Some(id);
                }
            }
            DeckbuilderAction::PlaySelected => play_selected(&mut model),
            DeckbuilderAction::EndTurn => end_turn(&mut model),
            DeckbuilderAction::Pause => model.paused = true,
            DeckbuilderAction::Resume => model.paused = false,
            DeckbuilderAction::ReturnToMenu => {
                model.screen = Screen::Menu;
                model.paused = false;
            }
        }
        dirty.0 = true;
    }
}

fn play_selected(model: &mut DeckbuilderModel) {
    let Some(selected) = model.selected else {
        return;
    };
    let Some(card) = model.cards.iter_mut().find(|card| card.id == selected) else {
        return;
    };
    if card.played
        || card.cost > model.energy
        || model.turns.current() != Some(&Participant::Player)
    {
        return;
    }
    model.energy -= card.cost;
    card.played = true;
    model
        .activity
        .push(format!("Player played {}.", card.title));
    model.selected = None;
}

fn end_turn(model: &mut DeckbuilderModel) {
    let previous = model.turns.current().copied();
    let Ok(transition) = model.turns.advance() else {
        model.activity.push("No participant can act.".to_owned());
        return;
    };
    model.selected = None;
    if transition.current == Participant::Player {
        model.energy = 3;
        for card in &mut model.cards {
            card.played = false;
        }
    }
    model.activity.push(format!(
        "{} ended their turn; {} begins.",
        previous.map_or("Unknown", Participant::label),
        transition.current.label()
    ));
}

fn mark_responsive_change(metrics: Res<ResolvedUiMetrics>, mut dirty: ResMut<UiDirty>) {
    if metrics.is_changed() {
        dirty.0 = true;
    }
}

fn render_if_dirty(
    mut commands: Commands,
    model: Res<DeckbuilderModel>,
    metrics: Res<ResolvedUiMetrics>,
    fonts: Res<UiFonts>,
    mut dirty: ResMut<UiDirty>,
    roots: Query<Entity, With<DeckbuilderUiRoot>>,
) {
    if !dirty.0 {
        return;
    }
    for root in &roots {
        commands.entity(root).try_despawn();
    }
    let view = project(&model);
    match view {
        DeckbuilderView::Menu => spawn_menu(&mut commands, &fonts),
        DeckbuilderView::Match(view) => spawn_match(&mut commands, &fonts, metrics.viewport, &view),
    }
    dirty.0 = false;
}

fn spawn_menu(commands: &mut Commands, fonts: &UiFonts) {
    commands
        .spawn((screen_root("Deckbuilder Menu"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(text(fonts, UiTextRole::Display, "Arcana Workshop"));
            root.spawn(text(
                fonts,
                UiTextRole::Supporting,
                "A small adopter, not a shared game engine.",
            ));
            spawn_action(root, fonts, "Start Match", DeckbuilderAction::Start, false);
        });
}

fn spawn_match(
    commands: &mut Commands,
    fonts: &UiFonts,
    viewport: UiViewportClass,
    view: &MatchView,
) {
    commands
        .spawn((screen_root("Deckbuilder Match"), DeckbuilderUiRoot))
        .with_children(|root| {
            let mut hud_region = root.spawn(region("Match HUD", UiRegionRole::Hud));
            hud_region
                .insert(Node {
                    width: Val::Percent(94.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|hud| {
                    hud.spawn((
                        Name::new("Turn Status"),
                        text(fonts, UiTextRole::Body, format!("Turn: {}", view.turn)),
                    ));
                    hud.spawn((
                        Name::new("Round Status"),
                        text(fonts, UiTextRole::Body, format!("Round: {}", view.round)),
                    ));
                    hud.spawn((
                        Name::new("Energy Status"),
                        text(fonts, UiTextRole::Body, format!("Energy: {}", view.energy)),
                    ));
                });

            root.spawn((
                Name::new("Match Content"),
                Node {
                    width: Val::Percent(94.0),
                    height: Val::Percent(72.0),
                    flex_direction: if viewport == UiViewportClass::Compact {
                        FlexDirection::Column
                    } else {
                        FlexDirection::Row
                    },
                    column_gap: Val::Px(18.0),
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            ))
            .with_children(|content| {
                let mut hand_panel = content.spawn(panel("Hand Panel"));
                hand_panel
                    .insert(Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    })
                    .with_children(|hand| {
                        hand.spawn(text(fonts, UiTextRole::Title, "Your Hand"));
                        let mut card_list =
                            hand.spawn(region("Card Scroll List", UiRegionRole::ScrollList));
                        card_list
                            .insert(Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(12.0),
                                row_gap: Val::Px(12.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            })
                            .with_children(|cards| {
                                for card_view in &view.cards {
                                    spawn_card(cards, fonts, card_view);
                                }
                            });
                    });

                let mut activity_panel_entity = content.spawn(panel("Activity Panel"));
                activity_panel_entity
                    .insert(Node {
                        width: if viewport == UiViewportClass::Compact {
                            Val::Percent(100.0)
                        } else {
                            Val::Px(320.0)
                        },
                        max_height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    })
                    .with_children(|activity_panel| {
                        activity_panel.spawn(text(fonts, UiTextRole::Title, "Activity"));
                        let mut activity_feed = activity_panel
                            .spawn(region("Activity Feed", UiRegionRole::ActivityFeed));
                        activity_feed
                            .insert(Node {
                                max_height: Val::Px(300.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            })
                            .with_children(|activity| {
                                for (index, line) in view.activity.iter().enumerate() {
                                    activity.spawn((
                                        Name::new(format!("Activity Line {index}")),
                                        text(fonts, UiTextRole::Supporting, line.clone()),
                                    ));
                                }
                            });
                    });
            });

            let mut action_rail = root.spawn(region("Action Rail", UiRegionRole::ActionRail));
            action_rail
                .insert(Node {
                    width: Val::Percent(94.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|actions| {
                    spawn_action(
                        actions,
                        fonts,
                        "Play Selected",
                        DeckbuilderAction::PlaySelected,
                        !view.can_play,
                    );
                    spawn_action(
                        actions,
                        fonts,
                        "End Turn",
                        DeckbuilderAction::EndTurn,
                        false,
                    );
                    spawn_action(actions, fonts, "Pause", DeckbuilderAction::Pause, false);
                });

            if view.paused {
                spawn_pause_modal(root, fonts);
            }
        });
}

fn spawn_card(parent: &mut ChildSpawnerCommands, fonts: &UiFonts, view: &CardView) {
    let mut card_entity = parent.spawn((
        card(format!("Card {}", view.title)),
        DeckbuilderAction::SelectCard(view.id),
        Button,
        bevy_game_ui::UiAction,
        bevy::input_focus::tab_navigation::TabIndex(0),
    ));
    if view.disabled {
        card_entity.insert(UiDisabled);
    }
    card_entity.with_children(|surface| {
        surface.spawn(text(fonts, UiTextRole::Title, view.title));
        surface.spawn(text(fonts, UiTextRole::Body, format!("Cost {}", view.cost)));
        surface.spawn(text(fonts, UiTextRole::Supporting, view.rules));
        if view.selected {
            surface.spawn((
                Name::new("Selected Card Indicator"),
                text(fonts, UiTextRole::Body, "SELECTED"),
            ));
        }
    });
}

fn spawn_action(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    name: &'static str,
    action: DeckbuilderAction,
    disabled: bool,
) {
    let mut entity = parent.spawn((button(name), action));
    if disabled {
        entity.insert(UiDisabled);
    }
    entity.with_child(text(fonts, UiTextRole::Body, name));
}

fn spawn_pause_modal(parent: &mut ChildSpawnerCommands, fonts: &UiFonts) {
    parent.spawn(modal("Pause Modal")).with_children(|overlay| {
        overlay.spawn(panel("Pause Panel")).with_children(|pause| {
            pause.spawn(text(fonts, UiTextRole::Title, "Paused"));
            spawn_action(pause, fonts, "Resume", DeckbuilderAction::Resume, false);
            spawn_action(
                pause,
                fonts,
                "Return to Menu",
                DeckbuilderAction::ReturnToMenu,
                false,
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input_focus::InputFocus;
    use bevy_game_test::{
        click_action, find_named, focus_action, run_frames, tap_key, ui_tree_snapshot,
        TestAppBuilder,
    };
    use bevy_game_ui::{UiScaleMode, UiScalePreference};

    fn test_app(width: u32, height: u32, scale: UiScaleMode) -> App {
        let mut builder = TestAppBuilder::new().with_ui(width, height);
        builder
            .app_mut()
            .insert_resource(UiScalePreference(scale))
            .add_plugins(DeckbuilderPlugin);
        builder.build()
    }

    fn start_match(app: &mut App) {
        run_frames(app, 3);
        let start = find_named(app.world_mut(), "Start Match")
            .expect("menu fixture must publish Start Match");
        assert!(click_action(app, start));
        run_frames(app, 3);
    }

    #[test]
    fn pointer_flow_selects_plays_and_advances_game_owned_state() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        start_match(&mut app);
        let spark = find_named(app.world_mut(), "Card Spark").expect("hand contains Spark");
        assert!(click_action(&mut app, spark));
        let play = find_named(app.world_mut(), "Play Selected").expect("action rail contains Play");
        assert!(click_action(&mut app, play));
        let model = app.world().resource::<DeckbuilderModel>();
        assert_eq!(model.energy, 2);
        assert!(model
            .cards
            .iter()
            .any(|card| card.title == "Spark" && card.played));

        let end = find_named(app.world_mut(), "End Turn").expect("action rail contains End Turn");
        assert!(click_action(&mut app, end));
        assert_eq!(
            app.world().resource::<DeckbuilderModel>().turns.current(),
            Some(&Participant::Opponent)
        );
    }

    #[test]
    fn keyboard_activation_matches_pointer_activation() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        run_frames(&mut app, 3);
        let start = find_named(app.world_mut(), "Start Match").expect("menu publishes Start Match");
        assert!(focus_action(app.world_mut(), start));
        tap_key(&mut app, KeyCode::Enter);
        run_frames(&mut app, 3);
        assert_eq!(
            app.world().resource::<DeckbuilderModel>().screen,
            Screen::Match
        );
    }

    #[test]
    fn disabled_cards_leave_focus_order_and_cannot_emit_game_actions() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        start_match(&mut app);
        let comet = find_named(app.world_mut(), "Card Comet").expect("hand contains Comet");
        assert!(app.world().get::<UiDisabled>(comet).is_some());
        assert_eq!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(comet)
                .map(|index| index.0),
            Some(-1)
        );
        assert!(!focus_action(app.world_mut(), comet));
        assert!(click_action(&mut app, comet));
        assert_eq!(app.world().resource::<DeckbuilderModel>().selected, None);
    }

    #[test]
    fn pause_modal_owns_and_restores_keyboard_focus() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        start_match(&mut app);
        let pause = find_named(app.world_mut(), "Pause").expect("action rail contains Pause");
        assert!(focus_action(app.world_mut(), pause));
        assert!(click_action(&mut app, pause));
        run_frames(&mut app, 3);
        let resume = find_named(app.world_mut(), "Resume").expect("pause modal contains Resume");
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(resume));
        assert!(click_action(&mut app, resume));
        run_frames(&mut app, 3);
        let restored_pause =
            find_named(app.world_mut(), "Pause").expect("match restores the Pause control");
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(restored_pause)
        );
    }

    #[test]
    fn responsive_matrix_has_structural_regions_and_minimum_targets() {
        for (width, height, scale) in [
            (1280, 720, UiScaleMode::Auto),
            (1920, 1080, UiScaleMode::Auto),
            (3840, 2160, UiScaleMode::Auto),
            (1280, 720, UiScaleMode::Percent200),
            (1920, 1080, UiScaleMode::Percent200),
            (3840, 2160, UiScaleMode::Percent200),
        ] {
            let mut app = test_app(width, height, scale);
            start_match(&mut app);
            let snapshot = ui_tree_snapshot(app.world_mut());
            let rendered = snapshot.to_string();
            assert!(rendered.contains("Match HUD [hud]"));
            assert!(rendered.contains("Action Rail [action-rail]"));
            assert!(rendered.contains("Activity Feed [activity-feed]"));
            for node in snapshot
                .nodes
                .iter()
                .filter(|node| node.action && !node.disabled)
            {
                assert!(node.size.x >= 44.0, "{} is too narrow", node.path);
                assert!(node.size.y >= 44.0, "{} is too short", node.path);
            }
            let mut semantic_text = app.world_mut().query::<(&UiTextRole, &TextFont)>();
            for (role, font) in semantic_text.iter(app.world()) {
                if matches!(role, UiTextRole::Body | UiTextRole::Supporting) {
                    let FontSize::Px(size) = font.font_size else {
                        continue;
                    };
                    assert!(size >= 18.0, "essential text resolved below 18px");
                }
            }
        }
    }
}

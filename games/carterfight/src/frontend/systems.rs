//! The game-owned input and authority boundary. Rendering never reads Runtime.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy_game_ui::{UiActivated, UiFonts, UiMotionPreference, UiScaleMode, UiScalePreference};

use super::{dialogue::UiNodes, sequencer::Runtime, CarterfightIntent, CarterfightView};

#[derive(Resource)]
pub(super) struct CarterAssets {
    pub(super) font: Handle<Font>,
    pub(super) carter: Handle<Image>,
    pub(super) cursor: Handle<Image>,
    pub(super) chime: Handle<AudioSource>,
}

pub(super) fn load_assets(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut fonts: ResMut<UiFonts>,
    provided: Option<Res<CarterAssets>>,
) {
    if let Some(assets) = provided {
        fonts.heading = assets.font.clone();
        fonts.body = assets.font.clone();
        return;
    }
    let font = server.load("fonts/PressStart2P-Regular.ttf");
    fonts.heading = font.clone();
    fonts.body = font.clone();
    commands.insert_resource(CarterAssets {
        font,
        carter: server.load("images/carter/boss_03_fighting_ready_256.png"),
        cursor: server.load("images/dialogue_cursor.png"),
        chime: server.load("sounds/dialogue_chime.wav"),
    });
}

pub(super) fn collect_input(
    mut activations: MessageReader<UiActivated>,
    actions: Query<&CarterfightIntent>,
    keys: Res<ButtonInput<KeyCode>>,
    view: Res<CarterfightView>,
    mut intents: MessageWriter<CarterfightIntent>,
) {
    // Read the entire batch without draining the shared bus; produce at most one
    // local decision. Other overlays can still independently observe activation.
    let clicked = activations
        .read()
        .filter_map(|event| actions.get(event.entity).ok())
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .next();
    let intent = if let Some(action) = clicked {
        Some(action)
    } else if keys.just_pressed(KeyCode::Escape) {
        Some(CarterfightIntent::CancelSelection)
    } else if view.can_select {
        let selected = [
            (KeyCode::Digit1, "jab"),
            (KeyCode::Digit2, "haymaker"),
            (KeyCode::Digit3, "headbutt"),
        ]
        .into_iter()
        .find(|(key, _)| keys.just_pressed(*key))
        .map(|(_, id)| CarterfightIntent::SelectMove(id));
        selected.or_else(|| {
            (keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter))
                .then_some(CarterfightIntent::ConfirmMove)
        })
    } else if keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::KeyZ)
    {
        Some(CarterfightIntent::Advance)
    } else {
        None
    };
    if let Some(intent) = intent {
        intents.write(intent);
    }
}

pub(super) fn apply_intent(
    mut intents: MessageReader<CarterfightIntent>,
    mut runtime: ResMut<Runtime>,
    mut scale: ResMut<UiScalePreference>,
    mut motion: ResMut<UiMotionPreference>,
    nodes: Res<UiNodes>,
    mut focus: ResMut<InputFocus>,
    mut exit: MessageWriter<AppExit>,
) {
    // All input paths arbitrate together, including synthetic native-review input.
    let intent = intents.read().next().cloned();
    // Consume any remaining same-frame input without letting it cross a phase.
    intents.read().for_each(|_| {});
    let Some(intent) = intent else {
        return;
    };
    match intent {
        CarterfightIntent::ToggleScale => {
            scale.0 = if scale.0 == UiScaleMode::Percent200 {
                UiScaleMode::Auto
            } else {
                UiScaleMode::Percent200
            }
        }
        CarterfightIntent::ToggleMotion => motion.reduced = !motion.reduced,
        CarterfightIntent::InspectMoves => {
            if runtime.can_select() {
                if let Some(entity) = nodes.moves.first() {
                    focus.set(*entity, FocusCause::Navigated);
                }
            }
        }
        _ => {
            if runtime.apply(&intent) {
                exit.write(AppExit::Success);
            }
            if matches!(intent, CarterfightIntent::SelectMove(_)) && runtime.pending.is_some() {
                focus.set(nodes.confirm, FocusCause::Navigated);
            } else if matches!(intent, CarterfightIntent::ConfirmMove) && !runtime.idle() {
                focus.set(nodes.advance, FocusCause::Navigated);
            }
        }
    }
}

pub(super) fn tick_narration(
    mut runtime: ResMut<Runtime>,
    time: Res<Time>,
    motion: Res<UiMotionPreference>,
    assets: Res<CarterAssets>,
    mut commands: Commands,
) {
    if runtime.tick(time.delta_secs(), motion.reduced) {
        // At most one short chime per frame, even after a slow frame. Never queue
        // an unbounded audio burst just because many characters became visible.
        commands.spawn((
            AudioPlayer::new(assets.chime.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}

pub(super) fn project(runtime: Res<Runtime>, mut view: ResMut<CarterfightView>) {
    let next = runtime.view();
    if *view != next {
        *view = next;
    }
}

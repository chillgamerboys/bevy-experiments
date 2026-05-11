use super::constants::*;
use bevy::prelude::*;
use std::collections::VecDeque;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueQueue>()
            .init_resource::<DialogueState>()
            .add_systems(Startup, setup_dialogue_box)
            .add_systems(Update, (tick_dialogue, dialogue_input, update_cursor).chain());
    }
}

#[derive(Resource, Default)]
pub struct DialogueQueue(pub VecDeque<String>);

impl DialogueQueue {
    pub fn push(&mut self, msg: impl Into<String>) {
        self.0.push_back(msg.into());
    }
}

#[derive(Resource)]
pub struct DialogueState {
    full_text: String,
    chars_shown: usize,
    char_timer: f32,
    secs_per_char: f32,
    pub is_done: bool,
    chime: Handle<AudioSource>,
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            full_text: String::new(),
            chars_shown: 0,
            char_timer: 0.0,
            secs_per_char: 1.0 / DIALOGUE_CHARS_PER_SEC,
            is_done: true,
            chime: Handle::default(),
        }
    }
}

#[derive(Component)]
struct DialogueText;

#[derive(Component)]
struct DialogueCursor;

fn setup_dialogue_box(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<DialogueState>,
) {
    state.chime = asset_server.load("sounds/dialogue_chime.wav");
    let cursor_image: Handle<Image> = asset_server.load("images/dialogue_cursor.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(DIALOGUE_BOX_MARGIN),
                left: Val::Px(DIALOGUE_BOX_MARGIN),
                right: Val::Px(DIALOGUE_BOX_MARGIN),
                height: Val::Px(DIALOGUE_BOX_HEIGHT),
                border: UiRect::all(Val::Px(DIALOGUE_BOX_BORDER)),
                padding: UiRect::all(Val::Px(DIALOGUE_BOX_PADDING)),
                border_radius: bevy::ui::BorderRadius::all(Val::Px(DIALOGUE_BOX_RADIUS)),
                ..default()
            },
            BackgroundColor(DIALOGUE_BOX_BG),
            BorderColor::all(DIALOGUE_BOX_BORDER_COLOR),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: asset_server.load(FONT_PATH),
                    font_size: DIALOGUE_BOX_FONT_SIZE,
                    ..default()
                },
                TextColor(DIALOGUE_BOX_TEXT_COLOR),
                DialogueText,
            ));

            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(DIALOGUE_CURSOR_OFFSET),
                    right: Val::Px(DIALOGUE_CURSOR_OFFSET),
                    width: Val::Px(DIALOGUE_CURSOR_W),
                    height: Val::Px(DIALOGUE_CURSOR_H),
                    ..default()
                },
                ImageNode::new(cursor_image),
                DialogueCursor,
            ));
        });
}

fn tick_dialogue(
    mut queue: ResMut<DialogueQueue>,
    mut state: ResMut<DialogueState>,
    mut text_q: Query<&mut Text, With<DialogueText>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    // Only load the next queued message when the box is empty. A finished
    // message stays on screen as a prompt until the user presses space (see
    // `dialogue_input`), which clears `full_text` and lets us advance here.
    if state.is_done {
        if state.full_text.is_empty() {
            if let Some(msg) = queue.0.pop_front() {
                state.full_text = msg;
                state.chars_shown = 0;
                state.char_timer = 0.0;
                state.is_done = false;
                if let Ok(mut text) = text_q.single_mut() {
                    **text = String::new();
                }
            }
        }
        return;
    }

    let total = state.full_text.chars().count();
    state.char_timer += time.delta_secs();

    while state.char_timer >= state.secs_per_char && state.chars_shown < total {
        state.char_timer -= state.secs_per_char;
        state.chars_shown += 1;
        commands.spawn((
            AudioPlayer::new(state.chime.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }

    let shown: String = state.full_text.chars().take(state.chars_shown).collect();
    if state.chars_shown >= total {
        state.is_done = true;
    }

    if let Ok(mut text) = text_q.single_mut() {
        **text = shown;
    }
}

fn update_cursor(
    state: Res<DialogueState>,
    mut cursor_q: Query<&mut Visibility, With<DialogueCursor>>,
) {
    if let Ok(mut vis) = cursor_q.single_mut() {
        *vis = if state.is_done && !state.full_text.is_empty() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn dialogue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DialogueState>,
    queue: Res<DialogueQueue>,
    mut text_q: Query<&mut Text, With<DialogueText>>,
) {
    let pressed =
        keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyZ);
    if !pressed {
        return;
    }
    if !state.is_done {
        state.chars_shown = state.full_text.chars().count();
        state.is_done = true;
        let shown = state.full_text.clone();
        if let Ok(mut text) = text_q.single_mut() {
            **text = shown;
        }
    } else if !state.full_text.is_empty() && !queue.0.is_empty() {
        // Advance — clear current message so tick_dialogue can load the next.
        // If the queue is empty, this is the final queued message — keep it on
        // screen as a sticky prompt until something new gets pushed.
        state.full_text.clear();
        if let Ok(mut text) = text_q.single_mut() {
            **text = String::new();
        }
    }
}

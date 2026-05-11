use bevy::prelude::*;
use std::collections::VecDeque;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueQueue>()
            .init_resource::<DialogueState>()
            .add_systems(Startup, setup_dialogue_box)
            .add_systems(Update, (tick_dialogue, dialogue_input).chain());
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
    audio_ready: bool, // browsers block audio until first user gesture
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            full_text: String::new(),
            chars_shown: 0,
            char_timer: 0.0,
            secs_per_char: 1.0 / 20.0,
            is_done: true,
            chime: Handle::default(),
            audio_ready: false,
        }
    }
}

#[derive(Component)]
struct DialogueText;

fn setup_dialogue_box(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<DialogueState>,
) {
    state.chime = asset_server.load("sounds/dialogue_chime.wav");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                height: Val::Px(160.0),
                border: UiRect::all(Val::Px(4.0)),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::WHITE),
            BorderColor::all(Color::BLACK),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(Color::BLACK),
                DialogueText,
            ));
        });
}

fn tick_dialogue(
    mut queue: ResMut<DialogueQueue>,
    mut state: ResMut<DialogueState>,
    mut text_q: Query<&mut Text, With<DialogueText>>,
    time: Res<Time>,
) {
    // When the current message is fully typed, advance to the next queued one
    // (if any). This also handles the "sticky last message" case: the previous
    // message stays visible until something new gets pushed, at which point we
    // swap to the new content.
    if state.is_done {
        if let Some(msg) = queue.0.pop_front() {
            state.full_text = msg;
            state.chars_shown = 0;
            state.char_timer = 0.0;
            state.is_done = false;
            if let Ok(mut text) = text_q.single_mut() {
                **text = String::new();
            }
        }
        return;
    }

    let total = state.full_text.chars().count();
    state.char_timer += time.delta_secs();

    while state.char_timer >= state.secs_per_char && state.chars_shown < total {
        state.char_timer -= state.secs_per_char;
        state.chars_shown += 1;
        // TODO: re-enable audio once WASM audio context timing is sorted
        // if state.audio_ready
        //     && matches!(asset_server.load_state(&state.chime), bevy::asset::LoadState::Loaded)
        // {
        //     commands.spawn((
        //         AudioPlayer::new(state.chime.clone()),
        //         PlaybackSettings::DESPAWN,
        //     ));
        // }
    }

    let shown: String = state.full_text.chars().take(state.chars_shown).collect();
    if state.chars_shown >= total {
        state.is_done = true;
    }

    let display = if state.is_done {
        format!("{shown} ▼")
    } else {
        shown
    };

    if let Ok(mut text) = text_q.single_mut() {
        **text = display;
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
    state.audio_ready = true;

    if !state.is_done {
        // Skip typewriter — jump to end of current message
        state.chars_shown = state.full_text.chars().count();
        state.is_done = true;
        let display = format!("{} ▼", state.full_text);
        if let Ok(mut text) = text_q.single_mut() {
            **text = display;
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

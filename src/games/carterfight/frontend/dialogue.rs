use super::constants::*;
use super::super::backend::BattleEvent;
use super::sequencer::{advance_sequencer, AdvanceMode, Sequencer, SequencerPhase};
use bevy::prelude::*;
use std::collections::VecDeque;

/// One queued presentation step: an event and how its line advances after
/// typing finishes. Stored in [`BattleEventQueue`].
#[derive(Clone)]
pub struct QueuedEvent {
    pub event: BattleEvent,
    pub advance: AdvanceMode,
}

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BattleEventQueue>()
            .init_resource::<DialogueState>()
            .init_resource::<Sequencer>()
            .add_systems(Startup, setup_dialogue_box)
            .add_systems(
                Update,
                (advance_sequencer, tick_dialogue, dialogue_input, update_cursor).chain(),
            );
    }
}

/// Events waiting to be presented (visual side-effect + dialogue line). The
/// sequencer drains this one at a time.
#[derive(Resource, Default)]
pub struct BattleEventQueue(pub VecDeque<QueuedEvent>);

impl BattleEventQueue {
    /// Narration that the player should read — waits for Space after typing.
    pub fn push(&mut self, event: BattleEvent) {
        self.push_with(event, AdvanceMode::WaitForInput);
    }

    /// Prompt-style line — once typing finishes, auto-releases so the next
    /// system (input, state transition) can act without an extra ack press.
    pub fn push_auto(&mut self, event: BattleEvent) {
        self.push_with(event, AdvanceMode::AutoAfterTypewriter);
    }

    pub fn push_with(&mut self, event: BattleEvent, advance: AdvanceMode) {
        self.0.push_back(QueuedEvent { event, advance });
    }

    /// Convenience for non-combat scripted narration (intro/outro). Wraps the
    /// string as a `BattleEvent::Dialogue` so it flows through the same queue.
    pub fn push_line(&mut self, line: impl Into<String>) {
        self.push(BattleEvent::Dialogue(line.into()));
    }

    /// Same, but for prompt lines like "What will you do?".
    pub fn push_auto_line(&mut self, line: impl Into<String>) {
        self.push_auto(BattleEvent::Dialogue(line.into()));
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

impl DialogueState {
    pub fn full_text_is_empty(&self) -> bool {
        self.full_text.is_empty()
    }

    /// Called by the sequencer when it pops a new event. Resets the typewriter
    /// to the start of the new line.
    pub fn start(&mut self, text: String) {
        self.full_text = text;
        self.chars_shown = 0;
        self.char_timer = 0.0;
        self.is_done = false;
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

/// Pure typewriter: types out whatever `DialogueState` currently holds.
/// Popping the next message off the queue is `advance_sequencer`'s job.
fn tick_dialogue(
    mut state: ResMut<DialogueState>,
    mut text_q: Query<&mut Text, With<DialogueText>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    if state.is_done {
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
    sequencer: Res<Sequencer>,
    mut cursor_q: Query<&mut Visibility, With<DialogueCursor>>,
) {
    if let Ok(mut vis) = cursor_q.single_mut() {
        // Show the "press space" cursor only when the typewriter is finished
        // *and* the sequencer is actually waiting on input. Otherwise the
        // sticky final-line state would flash the cursor between events.
        let waiting_for_space = matches!(
            sequencer.phase,
            SequencerPhase::Presenting { advance: AdvanceMode::WaitForInput }
        );
        *vis = if state.is_done && !state.full_text.is_empty() && waiting_for_space {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn dialogue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DialogueState>,
    mut sequencer: ResMut<Sequencer>,
    mut text_q: Query<&mut Text, With<DialogueText>>,
) {
    let pressed =
        keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyZ);
    if !pressed {
        return;
    }
    if !state.is_done {
        // Mid-typing: skip to the end of the current line.
        state.chars_shown = state.full_text.chars().count();
        state.is_done = true;
        let shown = state.full_text.clone();
        if let Ok(mut text) = text_q.single_mut() {
            **text = shown;
        }
        return;
    }
    // Typing finished. If the sequencer is gated on input, release it so the
    // next event can pop on the following frame. Otherwise (Idle, or
    // AutoAfterTypewriter) ignore — the press belongs to another system
    // (e.g. `battle_input`'s move-confirm).
    if let SequencerPhase::Presenting { advance: AdvanceMode::WaitForInput } = sequencer.phase {
        sequencer.phase = SequencerPhase::Idle;
    }
}

//! Carterfight-owned input arbitration, narration, immutable projection and skin.

mod dialogue;
mod sequencer;
mod systems;
#[cfg(test)]
mod tests;

use bevy::prelude::*;
use bevy_gamekit::ui::{GameUiSystems, UiMotionPreference};

use crate::backend::MoveId;

/// Coarse game flow; rules phases remain in the pure battle backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarterfightPhase {
    /// Authored encounter introduction.
    #[default]
    Intro,
    /// Move selection and event-by-event combat narration.
    Battle,
    /// Authored outcome and explicit exit.
    Outro,
}

/// Local intent; native controls and shortcuts take the same path.
#[derive(Message, Component, Debug, Clone, PartialEq, Eq)]
pub enum CarterfightIntent {
    /// Select without resolving a turn.
    SelectMove(MoveId),
    /// Resolve the already selected move.
    ConfirmMove,
    /// Discard an uncommitted selection.
    CancelSelection,
    /// Focus and scroll to the first move without selecting or committing it.
    InspectMoves,
    /// Reveal a line, acknowledge a completed line, or explicitly close the outro.
    Advance,
    /// Switch Auto and 200 percent semantic UI scaling.
    ToggleScale,
    /// Toggle the shared reduced-motion preference.
    ToggleMotion,
    /// Toggle only this game's dialogue chime.
    ToggleSound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoveView {
    id: MoveId,
    name: &'static str,
    selected: bool,
}

/// Immutable, non-authoritative presentation snapshot. HP follows narration.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct CarterfightView {
    /// Current intro/battle/outro presentation flow.
    pub phase: CarterfightPhase,
    /// Displayed player HP, not prematurely updated authoritative HP.
    pub player_hp: u16,
    /// Displayed Carter HP.
    pub opponent_hp: u16,
    /// Player maximum HP.
    pub player_max: u16,
    /// Carter maximum HP.
    pub opponent_max: u16,
    /// Authoritative completed turn count.
    pub turn: u32,
    /// Whether the local player may choose a move.
    pub can_select: bool,
    /// Whether a selected move may be committed.
    pub can_confirm: bool,
    /// Whether reveal/continue/close has a meaning at this instant.
    pub can_advance: bool,
    /// Revealed portion of the current narration.
    pub narration: String,
    /// Whether this line is still typing.
    pub typing: bool,
    moves: Vec<MoveView>,
    selected_description: String,
    continue_label: &'static str,
    sound: bool,
}

/// Public ordering seams for deterministic fixtures and application composition.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarterfightSystems {
    /// Reads native activations and contextual shortcuts.
    Input,
    /// Applies one action and advances local narration.
    Update,
    /// Projects and paints from immutable presentation data.
    Present,
}

/// Installs this game's rules, narration and native scene. Add Gamekit UI separately.
pub struct CarterfightPlugin;

impl Plugin for CarterfightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<sequencer::Runtime>()
            .init_resource::<CarterfightView>()
            .init_resource::<UiMotionPreference>()
            .add_message::<CarterfightIntent>()
            .configure_sets(
                Update,
                (
                    CarterfightSystems::Input,
                    CarterfightSystems::Update,
                    CarterfightSystems::Present,
                )
                    .chain()
                    .after(GameUiSystems::EmitActivations),
            )
            .add_systems(Startup, (systems::load_assets, dialogue::mount).chain())
            .add_systems(
                Update,
                systems::collect_input.in_set(CarterfightSystems::Input),
            )
            .add_systems(
                Update,
                (systems::apply_intent, systems::tick_narration)
                    .chain()
                    .in_set(CarterfightSystems::Update),
            )
            .add_systems(
                Update,
                (systems::project, dialogue::present)
                    .chain()
                    .in_set(CarterfightSystems::Present),
            )
            .add_systems(
                PostUpdate,
                dialogue::position_scene.after(bevy::ui::UiSystems::Layout),
            );
    }
}

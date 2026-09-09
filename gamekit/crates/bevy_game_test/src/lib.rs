//! Deterministic, capability-based test infrastructure for Bevy games.
//!
//! The helpers share mechanics rather than game fixtures. Owning games remain
//! responsible for their domain setup, assertions, and acceptance criteria.

use std::fmt;
use std::time::Duration;

use bevy::app::PluginsState;
use bevy::prelude::*;

#[cfg(feature = "ui")]
mod ui;
#[cfg(feature = "ui")]
pub use ui::*;

/// Default deterministic duration advanced by each minimal-app update.
pub const DEFAULT_FIXED_STEP: Duration = Duration::from_millis(100);

/// Builds a deterministic app from explicit capabilities.
pub struct TestAppBuilder {
    app: App,
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAppBuilder {
    /// Starts with a bare app and no hidden plugins or resources.
    #[must_use]
    pub fn new() -> Self {
        Self { app: App::new() }
    }

    /// Adds Bevy's minimal plugins and deterministic manual clock.
    #[must_use]
    pub fn with_minimal_plugins(mut self) -> Self {
        self.app.add_plugins(MinimalPlugins).insert_resource(
            bevy::time::TimeUpdateStrategy::ManualDuration(DEFAULT_FIXED_STEP),
        );
        self
    }

    /// Selects the deterministic duration advanced by every update.
    #[must_use]
    pub fn with_fixed_step(mut self, duration: Duration) -> Self {
        self.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(duration));
        self
    }

    /// Adds a renderer-free UI stack with an exact logical canvas.
    #[must_use]
    #[cfg(feature = "ui")]
    pub fn with_ui(mut self, width: u32, height: u32) -> Self {
        self.app.add_plugins(HeadlessUiPlugin::new(width, height));
        self
    }

    /// Gives the owning test access before plugin finalization.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// Finalizes plugins and returns a runnable app.
    pub fn build(mut self) -> App {
        if self.app.plugins_state() != PluginsState::Cleaned {
            while self.app.plugins_state() == PluginsState::Adding {
                #[cfg(not(target_arch = "wasm32"))]
                bevy::tasks::tick_global_task_pools_on_main_thread();
            }
            self.app.finish();
            self.app.cleanup();
        }
        self.app
    }
}

/// Failure returned when a bounded deterministic run never satisfies its condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunLimitExceeded {
    /// Maximum frames that were executed.
    pub frames: usize,
}

impl fmt::Display for RunLimitExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "condition did not become true within {} deterministic frames",
            self.frames
        )
    }
}

impl std::error::Error for RunLimitExceeded {}

/// Advances exactly `frames` updates, flushing deferred commands each time.
pub fn run_frames(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

/// Advances until `done` succeeds or the frame bound is exhausted.
pub fn run_until(
    app: &mut App,
    frames: usize,
    mut done: impl FnMut(&mut World) -> bool,
) -> Result<usize, RunLimitExceeded> {
    for frame in 0..frames {
        if done(app.world_mut()) {
            return Ok(frame);
        }
        app.update();
    }
    if done(app.world_mut()) {
        Ok(frames)
    } else {
        Err(RunLimitExceeded { frames })
    }
}

/// Finds one named entity without exposing query boilerplate to a test.
#[must_use]
pub fn find_named(world: &mut World, expected: &str) -> Option<Entity> {
    let mut names = world.query::<(Entity, &Name)>();
    names
        .iter(world)
        .find_map(|(entity, name)| (name.as_str() == expected).then_some(entity))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "ui")]
    use bevy_game_ui::{button, screen_root};

    #[test]
    fn bounded_runner_reports_non_completion_as_data() {
        let mut app = TestAppBuilder::new().with_minimal_plugins().build();
        assert_eq!(
            run_until(&mut app, 3, |_| false),
            Err(RunLimitExceeded { frames: 3 })
        );
    }

    #[test]
    #[cfg(feature = "ui")]
    fn snapshots_use_names_instead_of_entity_ids() {
        let mut app = TestAppBuilder::new().with_ui(1280, 720).build();
        let child = app.world_mut().spawn(button("Start")).id();
        app.world_mut().spawn(screen_root("Menu")).add_child(child);
        run_frames(&mut app, 3);
        let snapshot = ui_tree_snapshot(app.world_mut()).to_string();
        assert!(snapshot.contains("Menu/Start [action]"));
        assert!(!snapshot.contains("Entity"));
    }
}

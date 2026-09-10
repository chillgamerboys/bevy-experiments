//! Native Labyrinth executable.

use bevy::prelude::*;
use bevy_gamekit::multiplayer::{AtomicFileReconnectCredentialStore, ReconnectCredentialStorage};
use bevy_gamekit::ui::GameUiPlugin;
use labyrinth::{
    profile::{LaunchError, LaunchOptions, ProfileGuard},
    ui::LabyrinthUiConfig,
    view::LabyrinthIntent,
    LabyrinthPlugin,
};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            use std::io::Write as _;
            let _written = writeln!(std::io::stderr(), "Labyrinth could not start: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = match LaunchOptions::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(LaunchError::HelpRequested) => {
            use std::io::Write as _;
            let _written = writeln!(std::io::stdout(), "{}", LaunchError::HelpRequested);
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };
    let profile = ProfileGuard::begin(&options).map_err(|e| e.to_string())?;
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: profile.title(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ReconnectCredentialStorage::new(
        AtomicFileReconnectCredentialStore::new(profile.credential_path()),
    ))
    .insert_resource(LabyrinthUiConfig { seed: options.seed })
    .add_plugins((GameUiPlugin, LabyrinthPlugin))
    .add_systems(Startup, |mut commands: Commands| {
        commands.spawn((Camera2d, IsDefaultUiCamera));
    });
    if options.local {
        let seed = options.seed;
        app.add_systems(
            Startup,
            move |mut intents: MessageWriter<LabyrinthIntent>| {
                intents.write(LabyrinthIntent::StartLocal(seed));
            },
        );
    }
    app.run();
    Ok(())
}

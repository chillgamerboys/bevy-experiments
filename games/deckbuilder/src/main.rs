//! Interactive executable for the deckbuilder Gamekit adopter.

use bevy::prelude::*;
use bevy_gamekit::ui::GameUiPlugin;
use deckbuilder::DeckbuilderPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gamekit Deckbuilder".to_owned(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((GameUiPlugin, DeckbuilderPlugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Camera2d, IsDefaultUiCamera));
        })
        .run();
}

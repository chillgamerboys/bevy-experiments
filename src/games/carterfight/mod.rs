use bevy::prelude::*;

mod dialogue;
use dialogue::{DialoguePlugin, DialogueQueue};

pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carterfight".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DialoguePlugin)
        .add_systems(Startup, (setup, queue_test_lines).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn queue_test_lines(mut queue: ResMut<DialogueQueue>) {
    queue.push("A wild CARTER appeared!");
    queue.push("What will you do?");
    queue.push("CARTER used SICK BEAT! It's super effective!");
}

//! Deterministic static-frame evidence; deliberately no transport or gameplay claim.

use std::path::PathBuf;

use bevy::render::{
    render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
};
use bevy::{app::AppExit, asset::RenderAssetUsages, camera::RenderTarget, prelude::*};
use bevy_game_ui::{
    resolve_ui_metrics, GameUiPlugin, GameUiSystems, ResolvedUiMetrics, UiScaleMode,
    UiScalePreference,
};
use labyrinth::{
    ui::LabyrinthUiPlugin,
    view::{LabyrinthView, PlayerView, ViewMode},
};
use labyrinth_rules::{Combat, HeroClass};

#[derive(Resource)]
struct Capture {
    output: PathBuf,
    frame: u32,
    requested: bool,
    size: UVec2,
    scale: UiScaleMode,
    target: Option<Handle<Image>>,
    route: String,
    release: Option<&'static str>,
}

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let output = arguments.get(1).map_or_else(
        || PathBuf::from("target/labyrinth-review.png"),
        PathBuf::from,
    );
    let width = arguments
        .get(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1920);
    let height = arguments
        .get(3)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1080);
    let scale = if arguments.get(4).is_some_and(|value| value == "200") {
        UiScaleMode::Percent200
    } else {
        UiScaleMode::Auto
    };
    let route = arguments
        .get(5)
        .cloned()
        .unwrap_or_else(|| "combat".to_owned());
    let mut combat = Combat::new(42, HeroClass::ALL).expect("review fixture");
    for _ in 0..8 {
        let Some(action) = combat.ai_action() else {
            break;
        };
        if let Some(actor) = combat.snapshot().active_actor {
            combat.apply(actor, action).expect("legal fixture AI");
        }
    }
    let view = LabyrinthView {
        mode: match route.as_str() {
            "menu" | "host" => ViewMode::Menu,
            "lobby" => ViewMode::Lobby,
            _ => ViewMode::Combat,
        },
        local: !matches!(route.as_str(), "lobby" | "paused"),
        host: true,
        admitted: true,
        player: Some(0),
        encounter: 1,
        session_name: "The Lantern Company".to_owned(),
        combat: Some(combat.snapshot()),
        paused: route == "paused",
        players: HeroClass::ALL
            .into_iter()
            .enumerate()
            .map(|(index, hero)| PlayerView {
                slot: u8::try_from(index).expect("four-player index"),
                hero,
                name: format!("Player {}", index + 1),
                occupied: true,
                connected: !(route == "paused" && index == 2),
                ready: index != 2,
            })
            .collect(),
        invite_labels: (1..=3)
            .map(|index| format!("Guest {index} | unused invitation"))
            .collect(),
        log: vec![
            "The company entered the breach.".to_owned(),
            "A new initiative order was rolled.".to_owned(),
        ],
        ..LabyrinthView::default()
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Labyrinth Static Review".to_owned(),
                resolution: (width, height).into(),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(UiScalePreference(scale))
        .insert_resource(view)
        .insert_resource(Capture {
            output,
            frame: 0,
            requested: false,
            size: UVec2::new(width, height),
            scale,
            target: None,
            route,
            release: None,
        })
        .add_plugins((GameUiPlugin, LabyrinthUiPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            force_metrics
                .after(GameUiSystems::ResolveMetrics)
                .before(GameUiSystems::EmitActivations),
        )
        .add_systems(Update, capture.before(GameUiSystems::EmitActivations))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut capture: ResMut<Capture>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: capture.size.x,
            height: capture.size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image = images.add(image);
    commands.spawn((
        Camera2d,
        RenderTarget::Image(image.clone().into()),
        IsDefaultUiCamera,
    ));
    capture.target = Some(image);
}

fn force_metrics(capture: Res<Capture>, mut metrics: ResMut<ResolvedUiMetrics>) {
    let wanted = resolve_ui_metrics(capture.size.as_vec2(), capture.scale);
    if *metrics != wanted {
        *metrics = wanted;
    }
}

fn capture(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    mut controls: Query<(&Name, &mut Interaction), With<Button>>,
) {
    capture.frame += 1;
    if let Some(release) = capture.release.take() {
        for (name, mut interaction) in &mut controls {
            if name.as_str() == release {
                *interaction = Interaction::None;
            }
        }
    }
    let click = match (capture.route.as_str(), capture.frame) {
        ("host", 4) => Some("Host Company"),
        ("combat", 4) => Some("Skill 0"),
        ("combat", 7) => Some("Actor 101"),
        _ => None,
    };
    if let Some(click) = click {
        for (name, mut interaction) in &mut controls {
            if name.as_str() == click {
                *interaction = Interaction::Pressed;
                capture.release = Some(click);
            }
        }
    }
    if capture.frame >= 24 && !capture.requested {
        if let Some(target) = capture.target.clone() {
            commands
                .spawn(Screenshot::image(target))
                .observe(save_to_disk(capture.output.clone()))
                .observe(
                    |_event: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                        exit.write(AppExit::Success);
                    },
                );
            capture.requested = true;
        }
    }
}

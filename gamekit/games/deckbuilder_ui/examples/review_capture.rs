//! Deterministic rendered-frame capture for deckbuilder presentation review.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use bevy_game_ui::{
    resolve_ui_metrics, GameUiPlugin, GameUiSystems, ResolvedUiMetrics, UiScaleMode,
    UiScalePreference,
};
use deckbuilder_ui::DeckbuilderPlugin;

#[derive(Resource)]
struct CapturePlan {
    output: PathBuf,
    frame: u32,
    release: Option<&'static str>,
    requested: bool,
    size: UVec2,
    scale: UiScaleMode,
    target: Option<Handle<Image>>,
    route: String,
}

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let output = arguments.get(1).map_or_else(
        || PathBuf::from("target/deckbuilder-review.png"),
        PathBuf::from,
    );
    let width = arguments
        .get(2)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1920);
    let height = arguments
        .get(3)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1080);
    let scale = if arguments.get(4).is_some_and(|value| value == "200") {
        UiScaleMode::Percent200
    } else {
        UiScaleMode::Auto
    };
    let route = arguments
        .get(5)
        .cloned()
        .unwrap_or_else(|| "match".to_owned());

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Deckbuilder Review Capture".to_owned(),
                resolution: (width, height).into(),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(UiScalePreference(scale))
        .insert_resource(CapturePlan {
            output,
            frame: 0,
            release: None,
            requested: false,
            size: UVec2::new(width, height),
            scale,
            target: None,
            route,
        })
        .add_plugins((GameUiPlugin, DeckbuilderPlugin))
        .add_systems(Startup, setup_capture_target)
        .add_systems(
            Update,
            force_capture_metrics
                .after(GameUiSystems::ResolveMetrics)
                .before(GameUiSystems::EmitActivations),
        )
        .add_systems(Update, drive_capture.before(GameUiSystems::EmitActivations))
        .run();
}

fn setup_capture_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut plan: ResMut<CapturePlan>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: plan.size.x,
            height: plan.size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let target = images.add(image);
    commands.spawn((
        Camera2d,
        RenderTarget::Image(target.clone().into()),
        IsDefaultUiCamera,
    ));
    plan.target = Some(target);
}

fn force_capture_metrics(plan: Res<CapturePlan>, mut metrics: ResMut<ResolvedUiMetrics>) {
    let wanted = resolve_ui_metrics(plan.size.as_vec2(), plan.scale);
    if *metrics != wanted {
        *metrics = wanted;
    }
}

fn drive_capture(
    mut commands: Commands,
    mut plan: ResMut<CapturePlan>,
    mut interactions: Query<(&Name, &mut Interaction), With<Button>>,
) {
    plan.frame += 1;
    if let Some(release) = plan.release.take() {
        for (name, mut interaction) in &mut interactions {
            if name.as_str() == release {
                *interaction = Interaction::None;
            }
        }
    }

    let click = match (plan.route.as_str(), plan.frame) {
        ("match", 3) => Some("Start Solo"),
        ("match", 7) => Some("Card Spark"),
        ("multiplayer" | "host" | "browser", 3) => Some("Multiplayer"),
        ("host", 7) => Some("Host Session"),
        ("browser", 7) => Some("Find Sessions"),
        _ => None,
    };
    if let Some(click) = click {
        for (name, mut interaction) in &mut interactions {
            if name.as_str() == click {
                *interaction = Interaction::Pressed;
                plan.release = Some(click);
            }
        }
    }

    if plan.frame >= 20 && !plan.requested {
        let output = plan.output.clone();
        let Some(target) = plan.target.clone() else {
            return;
        };
        commands
            .spawn(Screenshot::image(target))
            .observe(save_to_disk(output))
            .observe(
                |_captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
        plan.requested = true;
    }
}

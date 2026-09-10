//! Deterministic static presentation capture, not evidence of pointer picking or audio.

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
    },
};
use bevy_gamekit::ui::{
    resolve_ui_metrics, GameUiPlugin, GameUiSkinPlugin, GameUiSystems, ResolvedUiMetrics,
    UiMotionPreference, UiScaleMode, UiScalePreference,
};
use carterfight::{
    asset_root, CarterfightIntent, CarterfightPhase, CarterfightPlugin, CarterfightSystems,
    CarterfightView,
};
use std::{path::PathBuf, time::Duration};

#[derive(Resource)]
struct Capture {
    output: PathBuf,
    size: UVec2,
    scale: UiScaleMode,
    frame: u32,
    ready: Option<u32>,
    target: Option<Handle<Image>>,
    requested: bool,
    route: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let output = args.get(1).map_or_else(
        || PathBuf::from("target/review/carterfight-1080-auto.png"),
        PathBuf::from,
    );
    let width: u32 = args.get(2).map_or(Ok(1920), |value| value.parse())?;
    let height: u32 = args.get(3).map_or(Ok(1080), |value| value.parse())?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err("capture size must be 1..=8192".into());
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let scale = if args.get(4).is_some_and(|value| value == "200") {
        UiScaleMode::Percent200
    } else {
        UiScaleMode::Auto
    };
    let route = args.get(5).cloned().unwrap_or_else(|| "battle".to_owned());
    if !matches!(route.as_str(), "intro" | "battle" | "damage" | "outro") {
        return Err("route must be intro, battle, damage or outro".into());
    }
    let exit = App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Carterfight Static Review".to_owned(),
                        resolution: (width, height).into(),
                        visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.055, 0.08, 0.15)))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ))
        .insert_resource(UiScalePreference(scale))
        .insert_resource(UiMotionPreference { reduced: true })
        .insert_resource(Capture {
            output,
            size: UVec2::new(width, height),
            scale,
            frame: 0,
            ready: None,
            target: None,
            requested: false,
            route,
        })
        .add_plugins((GameUiPlugin, GameUiSkinPlugin, CarterfightPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            force_metrics
                .after(GameUiSystems::ResolveMetrics)
                .before(GameUiSystems::EmitActivations),
        )
        .add_systems(Update, capture.before(CarterfightSystems::Input))
        .run();
    if exit == AppExit::Success {
        Ok(())
    } else {
        Err("Carterfight capture did not complete".into())
    }
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
    let next = resolve_ui_metrics(capture.size.as_vec2(), capture.scale);
    if *metrics != next {
        *metrics = next;
    }
}

fn capture(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    view: Res<CarterfightView>,
    mut motion: ResMut<UiMotionPreference>,
    mut intents: MessageWriter<CarterfightIntent>,
    mut exit: MessageWriter<AppExit>,
) {
    capture.frame += 1;
    if capture.frame == 1 {
        intents.write(CarterfightIntent::ToggleSound);
    } else if capture.ready.is_none() {
        let ready = match capture.route.as_str() {
            "intro" => view.narration == "A wild CARTER appeared!",
            "battle" => view.can_confirm,
            "damage" => view.narration == "Carter took 20 damage.",
            "outro" => {
                view.phase == CarterfightPhase::Outro && view.narration == "You beat Carter!"
            }
            _ => false,
        };
        if ready {
            capture.ready = Some(capture.frame);
            motion.reduced = false;
        } else if view.can_confirm {
            intents.write(CarterfightIntent::ConfirmMove);
        } else if view.can_select {
            intents.write(CarterfightIntent::SelectMove("haymaker"));
        } else if view.can_advance {
            intents.write(CarterfightIntent::Advance);
        }
    }
    if capture
        .ready
        .is_some_and(|ready| capture.frame > ready + 24)
        && !capture.requested
    {
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
    if capture.frame > 300 {
        error!("Carterfight capture did not complete");
        exit.write(AppExit::error());
    }
}

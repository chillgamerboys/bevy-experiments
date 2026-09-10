//! Deterministic static-frame evidence; deliberately no transport or gameplay claim.

use std::path::PathBuf;

use bevy::render::{
    render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
};
use bevy::{app::AppExit, asset::RenderAssetUsages, camera::RenderTarget, prelude::*};
use bevy_gamekit::ui::{
    resolve_ui_metrics, GameUiPlugin, GameUiSystems, ResolvedUiMetrics, UiContextHelp,
    UiContextHelpState, UiContextHelpSystems, UiScaleMode, UiScalePreference,
};
use labyrinth::{
    presentation::{ActorDisclosure, CombatDisclosure},
    ui::{LabyrinthAppearance, LabyrinthUiPlugin},
    view::{CombatInterruption, LabyrinthView, PlayerView, PresentedEvent, ViewMode},
};
use labyrinth_rules::{
    ActorId, Combat, StatusInstance, StatusKind, DEFAULT_HERO_ROSTER, PARTY_SIZE,
};

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
    let large = matches!(
        route.as_str(),
        "footprints" | "corpses" | "corpse-forecast" | "corpse-help"
    );
    let heroes = if large {
        labyrinth_rules::PROTOTYPE_HERO_ROSTER.to_vec()
    } else {
        DEFAULT_HERO_ROSTER.to_vec()
    };
    let mut combat = if large {
        Combat::with_party(
            42,
            heroes
                .iter()
                .enumerate()
                .map(|(i, hero)| labyrinth_rules::HeroSetup::preset(ActorId(i as u16 + 1), *hero))
                .collect(),
        )
    } else {
        Combat::new(42, DEFAULT_HERO_ROSTER)
    }
    .expect("review fixture");
    let mut events = Vec::new();
    for _ in 0..PARTY_SIZE * 2 {
        let Some(action) = combat.ai_action() else {
            break;
        };
        if let Some(actor) = combat.snapshot().active_actor {
            events.extend(combat.apply(actor, action).expect("legal fixture AI"));
        }
    }
    if route == "corpse-forecast" {
        for _ in 0..24 {
            let actor = combat.snapshot().active_actor.expect("review decision");
            if actor == ActorId(3) {
                break;
            }
            events.extend(
                combat
                    .apply(actor, labyrinth_rules::CombatAction::Wait)
                    .expect("review wait"),
            );
        }
    }
    let mut snapshot = combat.snapshot();
    if matches!(route.as_str(), "corpse-forecast" | "corpse-help") {
        // Authored rare-state presentation fixture, not a recorded lifecycle walk.
        let actor = snapshot
            .actors
            .iter_mut()
            .find(|a| a.id == ActorId(103))
            .expect("Ash Brute");
        actor.hp = 0;
        actor.life = labyrinth_rules::LifeState::Corpse {
            hp: 1,
            max_hp: 5,
            created_round: snapshot.round,
        };
        actor.statuses = vec![StatusInstance {
            id: 900,
            kind: StatusKind::Bleed,
            source: ActorId(2),
            bearer: actor.id,
            potency: 2,
            remaining: 2,
            eligible_boundary: snapshot.boundary_sequence + 1,
        }];
        snapshot.validate().expect("corpse review state");
    }
    if route == "corpses" {
        // Authored visual fixture, not evidence of a death-save or damage transition.
        for actor in snapshot
            .actors
            .iter_mut()
            .filter(|a| [ActorId(5), ActorId(101)].contains(&a.id))
        {
            actor.hp = 0;
            actor.life = labyrinth_rules::LifeState::Corpse {
                hp: actor.max_hp.div_ceil(4),
                max_hp: actor.max_hp.div_ceil(4),
                created_round: snapshot.round,
            };
        }
        snapshot
            .validate()
            .expect("valid corpse presentation fixture");
    }
    if matches!(route.as_str(), "effects" | "inspect") {
        // Authored presentation fixture, not evidence that gameplay applied an effect.
        let boundary = snapshot.boundary_sequence;
        if let Some(actor) = snapshot
            .actors
            .iter_mut()
            .find(|actor| actor.id == ActorId(1))
        {
            for (id, kind, potency, remaining) in [
                (500, StatusKind::Bleed, 2, 3),
                (501, StatusKind::Brace, 2, 1),
            ] {
                actor.statuses.push(StatusInstance {
                    id,
                    kind,
                    source: ActorId(103),
                    bearer: actor.id,
                    potency,
                    remaining,
                    eligible_boundary: boundary + 1,
                });
            }
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
        combat: Some(snapshot),
        paused: route == "paused",
        interruption: if route == "paused" {
            CombatInterruption::WaitingForPlayers
        } else {
            CombatInterruption::None
        },
        events: events
            .into_iter()
            .map(|event| PresentedEvent {
                id: event.id,
                event,
            })
            .collect(),
        players: heroes
            .into_iter()
            .enumerate()
            .map(|(index, hero)| PlayerView {
                slot: u8::try_from(index).expect("six-player index"),
                actor: ActorId(u16::try_from(index + 1).expect("hero ID")),
                hero,
                name: format!("Player {}", index + 1),
                occupied: true,
                connected: !(route == "paused" && index == 2),
                ready: index != 2,
            })
            .collect(),
        invite_labels: (1..PARTY_SIZE)
            .map(|index| format!("Guest {index} | unused invitation"))
            .collect(),
        log: vec![
            "The company entered the breach.".to_owned(),
            "A new initiative order was rolled.".to_owned(),
        ],
        ..LabyrinthView::default()
    };
    let mut disclosure = CombatDisclosure::default();
    if route == "unknown" {
        for id in 101..=106 {
            disclosure.actors.insert(
                ActorId(id),
                ActorDisclosure {
                    health: false,
                    statuses: false,
                    details: false,
                },
            );
        }
    }
    let mut appearance = LabyrinthAppearance::default();
    if route == "alternate" {
        appearance.dock = Color::srgb(0.10, 0.17, 0.19);
        appearance.detail = Color::srgb(0.08, 0.14, 0.16);
        appearance.accent = Color::srgb(0.5, 0.95, 0.88);
        appearance.ink = Color::srgb(0.95, 0.96, 0.85);
        appearance.damage = Color::srgb(1.0, 0.65, 0.38);
    }
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
        .insert_resource(disclosure)
        .insert_resource(appearance)
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
        .add_systems(
            Update,
            help_fixture
                .after(UiContextHelpSystems::Resolve)
                .before(bevy_gamekit::ui::UiTooltipSystems::Resolve)
                .before(labyrinth::ui::LabyrinthUiSystems::Present),
        )
        .run();
}

// A render-target image has no native pointer/focus surface. Author the selected
// help state for pixel review; production input routing is verified separately.
fn help_fixture(
    capture: Res<Capture>,
    mut controls: Query<(Entity, &Name, &UiContextHelp, &mut Interaction)>,
    mut help: ResMut<UiContextHelpState>,
    mut settings: ResMut<bevy_gamekit::ui::UiTooltipSettings>,
) {
    let source = match capture.route.as_str() {
        "help" | "help-locked" => "Skill 0",
        "history-actor" => "Actor 105",
        "corpse-forecast" => "Actor 103",
        _ => return,
    };
    // Freeze a presentation state, not an input/timing test. Production timing
    // and native pointer dismissal are covered by the per-frame UI tests.
    settings.lock_delay = if capture.route == "help-locked" {
        std::time::Duration::ZERO
    } else {
        std::time::Duration::MAX
    };
    if let Some((entity, _, content, mut interaction)) = controls
        .iter_mut()
        .find(|(_, name, _, _)| name.as_str() == source)
    {
        *interaction = Interaction::Hovered;
        help.entity = Some(entity);
        help.content = Some(content.clone());
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
        ("host", 4) => Some("Multiplayer"),
        ("host", 7) => Some("Host Company"),
        ("game-menu", 4) => Some("Battle Settings"),
        ("leave", 4) => Some("Battle Settings"),
        ("leave", 7) => Some("Menu Leave"),
        ("settings", 4) => Some("Battle Settings"),
        ("settings", 7) => Some("Game Settings"),
        ("history" | "history-actor", 4) => Some("Battle Log Toggle"),
        ("combat" | "help", 4) => Some("Skill 0"),
        ("combat", 7) => Some("Actor 105"),
        ("unknown" | "alternate", 4) => Some("Skill 0"),
        ("unknown" | "alternate", 7) => Some("Actor 105"),
        ("inspect", 4) => Some("Actor 1 Effects"),
        ("corpse-help", 4) => Some("Actor 103 Effects"),
        ("corpse-forecast", 4) => Some("Skill 1"),
        ("corpse-forecast", 7) => Some("Actor 103"),
        ("order", 4) => Some("Initiative Actor 4"),
        ("compact", 4) => Some("Battle Log Toggle"),
        ("compact", 7) => Some("History Toggle"),
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

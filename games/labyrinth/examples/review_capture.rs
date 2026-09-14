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
        "footprints"
            | "sparse"
            | "corpses"
            | "corpse-forecast"
            | "corpse-help"
            | "movement"
            | "movement-blocked"
    );
    let catalog = labyrinth_rules::catalog::ContentCatalog::builtin().expect("catalog");
    let scenario = (matches!(route.as_str(), "lobby" | "abilities" | "ability-help")
        || route.starts_with("editor")
        || route.starts_with("construction"))
    .then(|| {
        let mut scenario = labyrinth_rules::scenario::Scenario::stock(
            labyrinth_rules::scenario::StockScenario::Prototype,
            42,
            &catalog,
        )
        .expect("review scenario");
        if matches!(route.as_str(), "abilities" | "ability-help") {
            let hero = scenario.heroes.first_mut().expect("captain");
            hero.actor.name = "Captain Lantern".into();
            hero.actor.base_speed = 100;
            hero.actor.build = labyrinth_rules::build::CharacterBuild {
                innate: catalog
                    .definition()
                    .abilities
                    .iter()
                    .take(12)
                    .map(|ability| labyrinth_rules::build::InnateGrant {
                        ability: ability.id.clone(),
                        provenance: labyrinth_rules::catalog::ContentId::new("captain_training")
                            .expect("provenance"),
                    })
                    .collect(),
                ..default()
            };
        }
        if route.starts_with("editor") {
            for actor in scenario.heroes.iter_mut().chain(&mut scenario.enemies) {
                actor.actor.build.weapon =
                    Some(labyrinth_rules::catalog::ContentId::new("dagger").expect("weapon"));
            }
        }
        scenario
    });
    let heroes = if let Some(scenario) = &scenario {
        scenario
            .heroes
            .iter()
            .map(|hero| match hero.actor.appearance {
                labyrinth_rules::ActorKind::Hero(class) => class,
                _ => unreachable!("stock heroes"),
            })
            .collect()
    } else if large {
        labyrinth_rules::PROTOTYPE_HERO_ROSTER.to_vec()
    } else {
        DEFAULT_HERO_ROSTER.to_vec()
    };
    let mut combat = if let Some(scenario) = &scenario {
        Combat::from_scenario(&catalog, scenario).expect("review scenario combat")
    } else if large {
        Combat::with_party(
            42,
            heroes
                .iter()
                .enumerate()
                .map(|(i, hero)| labyrinth_rules::HeroSetup::preset(ActorId(i as u16 + 1), *hero))
                .collect(),
        )
        .expect("review party")
    } else {
        Combat::new(42, DEFAULT_HERO_ROSTER).expect("review fixture")
    };
    let mut events = Vec::new();
    for _ in 0..PARTY_SIZE * 2 {
        if route.starts_with("movement") || route == "sparse" {
            break;
        }
        let Some(action) = combat.ai_action() else {
            break;
        };
        if let Some(actor) = combat.snapshot().active_actor {
            events.extend(combat.apply(actor, action).expect("legal fixture AI"));
        }
    }
    if route == "corpse-forecast" || route.starts_with("movement") || route == "sparse" {
        for _ in 0..24 {
            let actor = combat.snapshot().active_actor.expect("review decision");
            if actor
                == if route.starts_with("movement") || route == "sparse" {
                    ActorId(1)
                } else {
                    ActorId(3)
                }
            {
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
    if route == "sparse" {
        // Authored cleared-rank fixture; transition behavior has separate UI tests.
        let retained = [ActorId(1), ActorId(5), ActorId(101)];
        for actor in &mut snapshot.actors {
            if !retained.contains(&actor.id) {
                actor.hp = 0;
                actor.life = labyrinth_rules::LifeState::Removed;
                actor.statuses.clear();
            }
        }
        snapshot.hero_formation.retain(|id| retained.contains(id));
        snapshot.enemy_formation.retain(|id| retained.contains(id));
        snapshot.validate().expect("sparse review state");
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
    let company_snapshot = snapshot.clone();
    let mut view = LabyrinthView {
        mode: match route.as_str() {
            "menu" | "host" => ViewMode::Menu,
            route
                if route == "lobby"
                    || route.starts_with("editor")
                    || route.starts_with("construction") =>
            {
                ViewMode::Lobby
            }
            _ => ViewMode::Combat,
        },
        local: !matches!(route.as_str(), "lobby" | "paused" | "construction-owners"),
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
        assignment_revision: 1,
        setup_revision: 1,
        scenario,
        catalog: Some(catalog),
        company: heroes
            .iter()
            .copied()
            .enumerate()
            .map(|(index, hero)| {
                let actor = ActorId(u16::try_from(index + 1).expect("actor"));
                labyrinth::view::CompanyMember {
                    actor,
                    hero,
                    abilities: company_snapshot
                        .actor(actor)
                        .expect("company actor")
                        .abilities
                        .clone(),
                    owner: if matches!(route.as_str(), "lobby" | "paused") {
                        u8::try_from(index).expect("owner")
                    } else {
                        0
                    },
                }
            })
            .collect(),
        players: (0..6)
            .map(|index| PlayerView {
                slot: u8::try_from(index).expect("six-player index"),
                actors: if matches!(route.as_str(), "lobby" | "paused") {
                    if index < heroes.len() {
                        vec![ActorId(u16::try_from(index + 1).expect("hero ID"))]
                    } else {
                        Vec::new()
                    }
                } else if index == 0 {
                    (1..=u16::try_from(heroes.len()).expect("heroes"))
                        .map(ActorId)
                        .collect()
                } else {
                    Vec::new()
                },
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
    if route.starts_with("construction") {
        let scenario = view.scenario.as_mut().expect("construction scenario");
        let mut formation = labyrinth::view::LobbyFormation::compact(scenario);
        if matches!(
            route.as_str(),
            "construction-picker"
                | "construction-gap"
                | "construction-enemy"
                | "construction-owners"
        ) {
            // Authored sparse presentation, not evidence of an authority mutation.
            let team = if route == "construction-enemy" {
                &mut scenario.enemies
            } else {
                &mut scenario.heroes
            };
            let removed = team.remove(1).id;
            formation.heroes.retain(|p| p.actor != removed);
            formation.enemies.retain(|p| p.actor != removed);
        }
        if route == "construction-owners" {
            formation.hero_owners = [0, 1, 1, 2, 0, 0];
            for (player, name) in view
                .players
                .iter_mut()
                .zip(["Captain Rowan", "Mira", "Jules"])
            {
                player.name = name.into();
            }
            view.company
                .retain(|m| scenario.heroes.iter().any(|a| a.id == m.actor));
            for member in &mut view.company {
                member.owner = formation
                    .rank(member.actor)
                    .and_then(|rank| formation.owner(rank))
                    .unwrap_or(0);
            }
            for player in &mut view.players {
                player.actors = view
                    .company
                    .iter()
                    .filter(|m| m.owner == player.slot)
                    .map(|m| m.actor)
                    .collect();
            }
        }
        view.deployment_error = formation.deployment_error(scenario);
        view.formation = Some(formation);
    }
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
        "help" | "help-locked" | "ability-help" => "Skill 0",
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
    mut controls: Query<(Entity, &Name, &mut Interaction), With<Button>>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    mut scrolls: Query<(&Name, &mut ScrollPosition)>,
) {
    capture.frame += 1;
    if capture.frame == 6
        && capture.scale == UiScaleMode::Percent200
        && capture.route == "construction-picker"
    {
        // Authored scroll position for this static frame; native scrolling is a separate test.
        for (name, mut scroll) in &mut scrolls {
            if name.as_str() == "Character Type Picker" {
                scroll.x = 1000.0;
            }
        }
    }
    if capture.route.starts_with("editor") && capture.frame == 2 {
        for (entity, name, _) in &mut controls {
            let target = if capture.route == "editor-rank4" {
                "Edit Actor 4"
            } else {
                "Edit Actor 1"
            };
            if name.as_str() == target {
                focus.set(entity, bevy::input_focus::FocusCause::Navigated);
            }
        }
    }
    if capture.route == "editor-actions" && capture.frame == 8 {
        for (entity, name, _) in &mut controls {
            if name.as_str() == "Apply Build" {
                focus.set(entity, bevy::input_focus::FocusCause::Navigated);
            }
        }
    }
    if let Some(release) = capture.release.take() {
        for (_, name, mut interaction) in &mut controls {
            if name.as_str() == release {
                *interaction = Interaction::None;
            }
        }
    }
    let click = match (capture.route.as_str(), capture.frame) {
        ("editor-enemy", 2) => Some("Select Enemies Rank 1"),
        ("editor-enemy", 4) => Some("Edit Actor 103"),
        ("editor-rank4", 2) => Some("Select Heroes Rank 4"),
        ("editor-rank4", 4) => Some("Edit Actor 4"),
        ("editor-rank4", 7) => Some("Weapon dagger"),
        (route, 2) if route.starts_with("editor") => Some("Select Heroes Rank 1"),
        (route, 4) if route.starts_with("editor") => Some("Edit Actor 1"),
        ("editor-compare", 7) => Some("Weapon greatsword"),
        ("editor-detail", 7) => Some("Weapon dagger"),
        ("editor-learned", 7) => Some("Category Learned"),
        ("editor-learned", 10) => Some("Learned duelist_dagger_power"),
        ("editor-parameters", 7) => Some("Category Parameters"),
        ("construction", 4) => Some("Select Heroes Rank 1"),
        ("construction-picker" | "construction-gap" | "construction-owners", 4) => {
            Some("Select Heroes Rank 2")
        }
        ("construction-picker", 7) => Some("Inspect Type scout"),
        ("construction-owners", 7) => Some("Assign Selected Place"),
        ("construction-enemy", 4) => Some("Select Enemies Rank 2"),
        ("construction-enemy", 7) => Some("Inspect Type ossuary_hauler"),
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
        ("movement" | "movement-blocked", 4) => Some("Skill 2"),
        ("movement", 7) => Some("Actor 104"),
        ("movement-blocked", 7) => Some("Actor 103"),
        ("order", 4) => Some("Initiative Actor 4"),
        ("compact", 4) => Some("Battle Log Toggle"),
        ("compact", 7) => Some("History Toggle"),
        _ => None,
    };
    if let Some(click) = click {
        for (_, name, mut interaction) in &mut controls {
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

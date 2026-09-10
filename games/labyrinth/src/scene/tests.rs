//! Structural and geometry checks; these do not certify rendered presentation.

use super::*;
use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy_gamekit::testing::{run_frames, HeadlessUiPlugin};
use labyrinth_rules::{ActorKind, Combat, EnemyKind, HeroClass, DEFAULT_HERO_ROSTER};

fn close(actual: Vec2, expected: Vec2) {
    assert!(
        actual.abs_diff_eq(expected, 0.001),
        "{actual:?} != {expected:?}"
    );
}

fn camera(scale: u32) -> Camera {
    Camera {
        viewport: Some(Viewport {
            physical_position: UVec2::new(100, 50) * scale,
            physical_size: UVec2::new(800, 600) * scale,
            ..default()
        }),
        computed: ComputedCameraValues {
            clip_from_view: Mat4::orthographic_rh(-400.0, 400.0, -300.0, 300.0, -1000.0, 1000.0),
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(1000, 800) * scale,
                scale_factor: scale as f32,
            }),
            ..default()
        },
        ..default()
    }
}

#[test]
fn anchors_preserve_viewport_offset_device_scale_and_camera_pan_zoom() {
    for scale in [1, 2] {
        let camera = camera(scale);
        let scale = scale as f32;
        let frame = geometry::frame_from_pixels(
            Vec2::new(100.0, 200.0) * scale,
            scale.recip(),
            &UiGlobalTransform::from_translation(Vec2::new(250.0, 200.0) * scale),
            &camera,
            &GlobalTransform::from(
                Transform::from_xyz(50.0, 30.0, 0.0).with_scale(Vec3::splat(2.0)),
            ),
        )
        .expect("valid camera and UI geometry");
        close(frame.center, Vec2::new(-250.0, 230.0));
        close(frame.right, Vec2::new(200.0, 0.0));
        close(frame.up, Vec2::new(0.0, 400.0));
    }
}

#[test]
fn anchors_reject_singular_or_empty_geometry_and_preserve_reflection() {
    let camera = camera(1);
    let project = |size, ui: UiGlobalTransform| {
        geometry::frame_from_pixels(size, 1.0, &ui, &camera, &GlobalTransform::default())
    };
    assert!(project(Vec2::ZERO, UiGlobalTransform::default()).is_none());
    assert!(project(
        Vec2::splat(100.0),
        UiGlobalTransform::from_scale(Vec2::ZERO)
    )
    .is_none());
    let frame = project(
        Vec2::splat(100.0),
        UiGlobalTransform::from_scale(Vec2::new(-1.0, 1.0)),
    )
    .expect("reflection is invertible");
    close(frame.right, Vec2::new(-100.0, 0.0));
    assert_eq!(frame.transform(0.0, 0.0, 0.0).scale.y, -1.0);
}

#[test]
fn fixed_sheet_coordinates_are_catalog_specific_not_actor_id_specific() {
    let size = UVec2::new(1536, 1024);
    let appearance = SceneAppearance::default();
    let sheet_rect = |size, kind| appearance.actor_rect(size, kind);
    assert_eq!(
        sheet_rect(size, ActorKind::Hero(HeroClass::Gatekeeper)),
        Some(Rect::from_corners(Vec2::ZERO, Vec2::new(384.0, 512.0)))
    );
    assert_eq!(
        sheet_rect(size, ActorKind::Hero(HeroClass::FieldMedic)),
        Some(Rect::from_corners(
            Vec2::new(1196.0, 0.0),
            Vec2::new(1536.0, 512.0)
        ))
    );
    assert_eq!(
        sheet_rect(size, ActorKind::Enemy(EnemyKind::HollowArcher)),
        Some(Rect::from_corners(
            Vec2::new(1178.0, 512.0),
            Vec2::new(1536.0, 1024.0)
        ))
    );
    assert!(sheet_rect(UVec2::new(1535, 1024), ActorKind::Hero(HeroClass::Scout)).is_none());
    for kind in HeroClass::ALL.into_iter().map(ActorKind::Hero).chain([
        ActorKind::Enemy(EnemyKind::AshBrute),
        ActorKind::Enemy(EnemyKind::IronBrute),
        ActorKind::Enemy(EnemyKind::WoundStalker),
        ActorKind::Enemy(EnemyKind::HollowArcher),
    ]) {
        let rect = sheet_rect(size, kind).expect("bounded cell");
        assert!(rect.min.min_element() >= 0.0);
        assert!(rect.max.cmple(size.as_vec2()).all());
        assert!(rect.size().min_element() > 0.0);
    }
}

#[test]
fn environment_floor_tracks_feet_without_drawing_outside_the_stage() {
    let size = UVec2::new(1536, 1024);
    for target_floor in [0.15, 0.3, 0.5, 0.66, 0.9] {
        let rect = floor_aligned_rect(size, 0.66, target_floor).expect("valid source crop");
        assert!(rect.min.min_element() >= 0.0);
        assert!(rect.max.cmple(size.as_vec2()).all());
        let floor = (1024.0 * 0.66 - rect.min.y) / rect.height();
        assert!((floor - target_floor).abs() < 0.00001);
    }
    assert!(floor_aligned_rect(size, 0.0, 0.5).is_none());
    assert!(floor_aligned_rect(size, 0.66, f32::NAN).is_none());
}

fn fixture() -> (App, Entity) {
    let combat = Combat::new(7, DEFAULT_HERO_ROSTER)
        .expect("valid roster")
        .snapshot();
    let ids: Vec<_> = combat.actors.iter().map(|actor| actor.id).collect();
    let mut app = App::new();
    app.add_plugins(HeadlessUiPlugin::default())
        .insert_resource(LabyrinthView {
            mode: ViewMode::Combat,
            combat: Some(combat),
            ..default()
        })
        .add_plugins(LabyrinthScenePlugin);
    // Exercise deterministic geometric fallbacks, independent of async loading.
    app.world_mut()
        .resource_mut::<SceneAppearance>()
        .actor_sheet = None;
    app.world_mut()
        .resource_mut::<SceneAppearance>()
        .backdrop_image = None;
    let stage = app
        .world_mut()
        .spawn((
            SceneStageAnchor,
            Node {
                width: px(1200),
                height: px(600),
                column_gap: px(4),
                ..default()
            },
        ))
        .id();
    for actor in ids {
        let entity = app
            .world_mut()
            .spawn((
                SceneActorAnchor { actor },
                Button,
                Node {
                    width: px(80),
                    height: px(300),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut().entity_mut(stage).add_child(entity);
    }
    app.finish();
    app.cleanup();
    run_frames(&mut app, 5);
    (app, stage)
}

#[test]
fn sprite_identity_survives_snapshots_without_dirtying_unchanged_components() {
    let (mut app, _) = fixture();
    assert_eq!(app.world().resource::<SceneState>().actors.len(), 12);
    let body = app
        .world()
        .resource::<SceneState>()
        .actors
        .get(&ActorId(1))
        .expect("hero")
        .body;
    assert_eq!(
        app.world().get::<Visibility>(body),
        Some(&Visibility::Visible)
    );
    let changed = app
        .world()
        .entity(body)
        .get_ref::<Sprite>()
        .expect("sprite")
        .last_changed();
    run_frames(&mut app, 2);
    assert_eq!(
        app.world()
            .entity(body)
            .get_ref::<Sprite>()
            .expect("sprite")
            .last_changed(),
        changed
    );
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        snapshot.hero_formation.swap(0, 1);
        snapshot.revision += 1;
    }
    run_frames(&mut app, 2);
    assert_eq!(
        app.world()
            .resource::<SceneState>()
            .actors
            .get(&ActorId(1))
            .expect("hero")
            .body,
        body
    );
}

#[test]
fn hidden_stage_hides_sprites_and_menu_removes_scene_entities() {
    let (mut app, stage) = fixture();
    app.world_mut()
        .get_mut::<Node>(stage)
        .expect("stage")
        .display = Display::None;
    run_frames(&mut app, 1);
    assert!(app
        .world_mut()
        .query_filtered::<&Visibility, With<SceneOwned>>()
        .iter(app.world())
        .all(|visible| *visible == Visibility::Hidden));
    app.world_mut().resource_mut::<LabyrinthView>().mode = ViewMode::Menu;
    run_frames(&mut app, 1);
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<SceneOwned>>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(app.world().resource::<SceneState>().actors.is_empty());
}

#[test]
fn plugin_tolerates_no_renderer_asset_server_or_ui_resources() {
    let mut app = App::new();
    app.insert_resource(LabyrinthView::default())
        .add_plugins(LabyrinthScenePlugin);
    app.update();
    assert!(app
        .world()
        .resource::<SceneAppearance>()
        .actor_sheet
        .is_none());
}

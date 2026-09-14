//! Preserve the initial projection when independently ordered channels cross.

use super::*;

#[derive(Resource, Default)]
struct HeldAdmission {
    welcomes: Vec<DeckWelcome>,
    snapshots: Vec<DeckSnapshot>,
}

fn hold_admission(
    mut welcomes: ResMut<Messages<DeckWelcome>>,
    mut snapshots: ResMut<Messages<DeckSnapshot>>,
    mut held: ResMut<HeldAdmission>,
) {
    held.welcomes.extend(welcomes.drain());
    held.snapshots.extend(snapshots.drain());
}

#[test]
fn snapshot_before_welcome_waits_for_persisted_matching_admission() {
    let mut host = host("");
    let mut guest = socket_app();
    guest.init_resource::<HeldAdmission>().add_systems(
        PreUpdate,
        hold_admission
            .before(receive_welcome)
            .in_set(bevy_gamekit::multiplayer::MultiplayerSystems::Receive),
    );
    start_direct_join(
        guest.world_mut(),
        &hosted_code(host.world()).expect("invitation"),
    )
    .expect("real encrypted join");
    assert!(
        pump_until(
            &mut host,
            &mut guest,
            Duration::from_secs(10),
            |_, guest| {
                let held = guest.world().resource::<HeldAdmission>();
                !held.welcomes.is_empty() && !held.snapshots.is_empty()
            }
        ),
        "host admission did not reach the receive boundary: {}",
        diagnostics(&guest)
    );
    let world = guest.world_mut();
    let mut held = world
        .remove_resource::<HeldAdmission>()
        .expect("captured messages");
    let welcome = held.welcomes.pop().expect("host welcome");
    let envelope = held.snapshots.pop().expect("host snapshot");
    let expected = envelope.snapshot.clone();
    let mut stale = envelope.clone();
    stale.attempt = SessionId::generate();
    stale.snapshot.own_hand.clear();

    world.write_message(envelope);
    world
        .run_system_cached(receive_snapshots)
        .expect("early snapshot boundary");
    assert!(!world.resource::<DeckNetworkState>().admitted);
    assert!(world.resource::<DeckNetworkState>().latest.is_none());
    world.write_message(stale);
    world
        .run_system_cached(receive_snapshots)
        .expect("stale snapshot boundary");
    world.write_message(welcome);
    world
        .run_system_cached(receive_welcome)
        .expect("welcome boundary");
    world
        .run_system_cached(receive_snapshots)
        .expect("admitted snapshot boundary");
    assert!(world.resource::<DeckNetworkState>().admitted);
    assert!(
        world.resource::<DeckNetworkState>().latest.as_ref() == Some(&expected),
        "the matching welcome must recover an already-received private snapshot"
    );
}

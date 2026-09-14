//! Order the separately delivered admission and snapshot messages explicitly.

use super::*;

#[derive(Resource, Default)]
struct HeldAdmission {
    admissions: Vec<Admitted>,
    snapshots: Vec<SnapshotEnvelope>,
}

fn hold_admission(
    mut admissions: ResMut<Messages<Admitted>>,
    mut snapshots: ResMut<Messages<SnapshotEnvelope>>,
    mut held: ResMut<HeldAdmission>,
) {
    held.admissions.extend(admissions.drain());
    held.snapshots.extend(snapshots.drain());
}

fn captured_join() -> (Vec<App>, Admitted, SnapshotEnvelope) {
    let mut apps = vec![socket_app(None), socket_app(None)];
    app(&mut apps, 1)
        .init_resource::<HeldAdmission>()
        .add_systems(
            PreUpdate,
            hold_admission
                .after(MultiplayerSystems::Receive)
                .before(MultiplayerSystems::GameAuthority),
        );
    open_host(&mut apps, "");
    let code = hosted_code(app(&mut apps, 0).world(), 0).expect("invitation");
    start::join_code(app(&mut apps, 1).world_mut(), &code).expect("real join");
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| {
            let held = app(apps, 1).world().resource::<HeldAdmission>();
            !held.admissions.is_empty() && !held.snapshots.is_empty()
        }),
        "host must acknowledge the persisted offer and send its initial snapshot: {}",
        admission_diagnostics(&apps)
    );
    let world = app(&mut apps, 1).world_mut();
    let mut held = world
        .remove_resource::<HeldAdmission>()
        .expect("captured messages");
    let admitted = held.admissions.pop().expect("host admission");
    let envelope = held.snapshots.pop().expect("host snapshot");
    (apps, admitted, envelope)
}

#[test]
fn snapshot_arriving_before_admission_waits_for_the_matching_admission() {
    let (mut apps, admitted, envelope) = captured_join();
    let expected = envelope.snapshot.clone();
    let mut stale = envelope.clone();
    stale.attempt = SessionId::generate();
    stale.snapshot.players.clear();
    let world = app(&mut apps, 1).world_mut();

    // Replicon gives each message type its own ordered channel. Delivering a
    // snapshot one receive boundary before Admitted is a legal transport order.
    // Use the actual host-produced messages, after real encrypted offer/ACK.
    world.write_message(envelope);
    receive(world);
    assert!(!world.resource::<Runtime>().admitted);
    assert!(world.resource::<Runtime>().latest.is_none());
    // A later packet from another attempt must not replace the waiting state.
    world.write_message(stale);
    receive(world);

    world.write_message(admitted);
    receive(world);
    assert!(world.resource::<Runtime>().admitted);
    assert!(
        world.resource::<Runtime>().latest.as_ref() == Some(&expected),
        "admission must recover its already-received snapshot without another host revision"
    );
}

#[test]
fn buffered_snapshot_still_requires_the_admitted_peer_identity() {
    let (mut apps, admitted, mut envelope) = captured_join();
    envelope
        .snapshot
        .players
        .iter_mut()
        .find(|player| player.slot == admitted.slot)
        .expect("recipient")
        .peer = Some(PeerId::generate());
    let world = app(&mut apps, 1).world_mut();
    world.write_message(envelope);
    receive(world);
    assert!(world.resource::<Runtime>().latest.is_none());
    world.write_message(admitted);
    receive(world);
    let runtime = world.resource::<Runtime>();
    assert!(!runtime.admitted);
    assert!(runtime.connection.is_none());
    assert!(runtime.latest.is_none());
    assert!(runtime.pending_snapshot.is_none());
}

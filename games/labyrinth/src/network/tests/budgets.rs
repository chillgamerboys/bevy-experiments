//! Queue policy and actual encrypted ingress, with independent evidence for each.

use super::*;

fn join(apps: &mut [App], guest: usize) {
    let code = hosted_code(app(apps, 0).world(), guest - 1).expect("invitation");
    start::join_code(app(apps, guest).world_mut(), &code).expect("join");
    assert!(pump_until(apps, Duration::from_secs(10), |apps| app(
        apps, guest
    )
    .world()
    .resource::<Runtime>()
    .admitted));
}

fn connection(world: &World, slot: u8) -> Entity {
    *world
        .resource::<Hosted>()
        .connections
        .iter()
        .find(|(_, (_, s))| *s == slot)
        .expect("connection")
        .0
}

fn ready(sequence: u64, encounter: u64) -> GameRequest {
    GameRequest {
        sequence,
        encounter,
        decision: 0,
        command: SessionCommand::Ready(true),
    }
}

#[test]
fn request_bursts_preserve_quiet_progress_and_overflow_reconnect_watermark() {
    let directory = tempfile::tempdir().expect("profiles");
    let path = directory.path().join("noisy.json");
    let mut apps = vec![socket_app(None), socket_app(Some(&path)), socket_app(None)];
    open_default_host(&mut apps, "");
    join(&mut apps, 1);
    join(&mut apps, 2);
    let peer = stored(app(&mut apps, 1)).peer_id;
    let encounter = app(&mut apps, 0)
        .world()
        .resource::<PartyAuthority>()
        .snapshot(0)
        .encounter;
    // Actual encrypted delivery: both streams must survive, including a burst
    // larger than the per-frame dispatch budget.
    for sequence in 1..=128 {
        app(&mut apps, 1)
            .world_mut()
            .write_message(ready(sequence, encounter));
    }
    app(&mut apps, 2)
        .world_mut()
        .write_message(ready(1, encounter));
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| {
        let authority = app(apps, 0).world().resource::<PartyAuthority>();
        authority.next_sequence(1) == 129 && authority.next_sequence(2) == 2
    }));
    // Deterministically put the entire noisy prefix ahead of the quiet request
    // in the production host handler, independent of socket packet coalescing.
    let world = app(&mut apps, 0).world_mut();
    let noisy = connection(world, 1);
    let quiet = connection(world, 2);
    let stranger = world.spawn_empty().id();
    for sequence in 129..=256 {
        world.write_message(FromClient {
            client_id: ClientId::from(noisy),
            message: ready(sequence, encounter),
        });
    }
    for sequence in 1..=512 {
        world.write_message(FromClient {
            client_id: ClientId::from(stranger),
            message: ready(sequence, encounter),
        });
    }
    world.write_message(FromClient {
        client_id: ClientId::from(quiet),
        message: ready(2, encounter),
    });
    admission::host_messages(world);
    assert_eq!(world.resource::<PartyAuthority>().next_sequence(1), 145);
    assert_eq!(world.resource::<PartyAuthority>().next_sequence(2), 3);
    for _ in 0..7 {
        admission::host_messages(world);
    }
    assert_eq!(world.resource::<PartyAuthority>().next_sequence(1), 257);
    // More than capacity closes the physical connection without applying a
    // prefix, changing the reservation, or resetting sequence history.
    for sequence in 257..=385 {
        world.write_message(FromClient {
            client_id: ClientId::from(noisy),
            message: ready(sequence, encounter),
        });
    }
    world.write_message(FromClient {
        client_id: ClientId::from(quiet),
        message: ready(3, encounter),
    });
    admission::host_messages(world);
    assert!(
        world.get::<InboundRejected>(noisy).is_some()
            || world.get::<ConnectedClient>(noisy).is_none()
    );
    assert_eq!(world.resource::<PartyAuthority>().next_sequence(1), 257);
    assert_eq!(world.resource::<PartyAuthority>().next_sequence(2), 4);
    wait_guest_detached(&mut apps, peer);
    *app(&mut apps, 1) = socket_app(Some(&path));
    start::reconnect(app(&mut apps, 1).world_mut()).expect("reconnect");
    assert!(pump_until(&mut apps, Duration::from_secs(10), |apps| app(
        apps, 1
    )
    .world()
    .resource::<Runtime>()
    .admitted));
    assert_eq!(stored(app(&mut apps, 1)).peer_id, peer);
    assert_eq!(
        app(&mut apps, 1).world().resource::<Runtime>().sequence,
        257
    );
    app(&mut apps, 1)
        .world_mut()
        .write_message(ready(257, encounter));
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| app(
        apps, 0
    )
    .world()
    .resource::<PartyAuthority>()
    .next_sequence(1)
        == 258));
}

#[derive(Resource, Default)]
struct IngressEvidence {
    rejected: usize,
    decoded_hellos: usize,
}

fn record_rejected(_: On<Add, InboundRejected>, mut evidence: ResMut<IngressEvidence>) {
    evidence.rejected += 1;
}

fn record_hellos(
    mut hellos: MessageReader<FromClient<Hello>>,
    mut evidence: ResMut<IngressEvidence>,
) {
    evidence.decoded_hellos += hellos.read().count();
}

#[test]
fn encrypted_oversized_hello_is_rejected_before_decode_without_claiming_a_seat() {
    let mut apps = vec![socket_app(None), socket_app(None), socket_app(None)];
    app(&mut apps, 0)
        .init_resource::<IngressEvidence>()
        .add_observer(record_rejected)
        .add_systems(PreUpdate, record_hellos.in_set(MultiplayerSystems::Receive));
    open_default_host(&mut apps, "");
    let code = hosted_code(app(&mut apps, 0).world(), 0).expect("code");
    start::join_code(app(&mut apps, 1).world_mut(), &code).expect("physical join");
    app(&mut apps, 1)
        .world_mut()
        .resource_mut::<Runtime>()
        .credential = Some(Credential::Password(WirePassword("x".repeat(2048))));
    assert!(pump_until(&mut apps, Duration::from_secs(10), |apps| app(
        apps, 0
    )
    .world()
    .resource::<IngressEvidence>()
    .rejected
        == 1));
    let world = app(&mut apps, 0).world();
    assert_eq!(world.resource::<IngressEvidence>().decoded_hellos, 0);
    assert_eq!(world.resource::<Hosted>().security.reserved_peer_count(), 0);
    join(&mut apps, 2);
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<IngressEvidence>()
            .decoded_hellos,
        1
    );
}

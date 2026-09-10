//! Worker lifetime budgets, separate from live connection/receiver bookkeeping.

use super::*;

#[test]
fn cancelled_connection_receivers_do_not_release_in_flight_worker_permits() {
    assert_eq!(
        PASSWORD_WORKERS, 5,
        "all five remote players may verify concurrently"
    );
    let jobs = Arc::new(AtomicUsize::new(0));
    let mut outstanding = Vec::new();
    for _ in 0..PASSWORD_WORKERS {
        let permit = WorkerPermit::acquire(&jobs, PASSWORD_WORKERS).expect("available worker slot");
        let (sender, receiver) = mpsc::sync_channel::<bool>(1);
        drop(receiver); // Exact lifetime change caused by a disconnected connection.
        assert!(sender.send(false).is_err());
        outstanding.push(permit); // Work is still queued/running even without its UI recipient.
    }
    assert_eq!(jobs.load(Ordering::Acquire), PASSWORD_WORKERS);
    for _ in 0..100 {
        assert!(WorkerPermit::acquire(&jobs, PASSWORD_WORKERS).is_none());
    }
    outstanding.pop(); // Only actual worker completion releases capacity.
    assert_eq!(jobs.load(Ordering::Acquire), PASSWORD_WORKERS - 1);
    let replacement = WorkerPermit::acquire(&jobs, PASSWORD_WORKERS)
        .expect("one completed job permits one successor");
    assert!(WorkerPermit::acquire(&jobs, PASSWORD_WORKERS).is_none());
    drop(replacement);
    drop(outstanding);
    assert_eq!(jobs.load(Ordering::Acquire), 0);
}

#[test]
fn cancelled_host_preparation_cannot_start_another_worker_until_completion() {
    let budget = HostPreparationBudget::default();
    let permit = WorkerPermit::acquire(&budget.0, 1).expect("one host preparation");
    let (sender, receiver) = mpsc::sync_channel::<()>(1);
    drop(receiver);
    assert!(sender.send(()).is_err());
    assert!(WorkerPermit::acquire(&budget.0, 1).is_none());
    drop(permit);
    let next =
        WorkerPermit::acquire(&budget.0, 1).expect("finished cancelled job released capacity");
    assert_eq!(budget.0.load(Ordering::Acquire), 1);
    drop(next);
    assert_eq!(budget.0.load(Ordering::Acquire), 0);
}

#[test]
fn concurrent_worker_acquisition_never_exceeds_the_shared_limit() {
    let jobs = Arc::new(AtomicUsize::new(0));
    let ready = Arc::new(std::sync::Barrier::new(17));
    let release = Arc::new(std::sync::Barrier::new(17));
    let mut threads = Vec::new();
    for _ in 0..16 {
        let jobs = Arc::clone(&jobs);
        let ready = Arc::clone(&ready);
        let release = Arc::clone(&release);
        threads.push(std::thread::spawn(move || {
            let _permit = WorkerPermit::acquire(&jobs, PASSWORD_WORKERS);
            ready.wait();
            release.wait();
        }));
    }
    ready.wait();
    let admitted = jobs.load(Ordering::Acquire);
    release.wait();
    for thread in threads {
        thread.join().expect("worker budget fixture exits");
    }
    assert_eq!(admitted, PASSWORD_WORKERS);
    assert_eq!(jobs.load(Ordering::Acquire), 0);
}

fn minimal_app() -> App {
    let mut builder = bevy_game_test::TestAppBuilder::new().with_minimal_plugins();
    builder.app_mut().add_plugins(LabyrinthNetworkPlugin);
    builder.build()
}

#[test]
fn preparing_host_rejects_all_guest_routes_before_parse_or_connection_creation() {
    let mut app = minimal_app();
    discovery::browse_fake(app.world_mut());
    assert!(app.world().contains_resource::<discovery::Browser>());
    start::host(
        app.world_mut(),
        HostSettings {
            name: "Preparation lifecycle fixture".into(),
            password: SecretText::default(),
            address: "127.0.0.1".into(),
            port: 7777,
            lan: false,
            tailnet: false,
            seed: 42,
        },
    )
    .expect("prepare without opening a listener");
    assert!(
        !app.world().contains_resource::<discovery::Browser>(),
        "hosting closes the browser and its optional probing"
    );
    // No App update: even a completed background job has not been consumed.
    for result in [
        start::join_code(app.world_mut(), "not-a-code"),
        start::join_discovered(
            app.world_mut(),
            SessionId::from_bytes([5; 16]),
            "temporary-session-passphrase".into(),
        ),
        start::reconnect(app.world_mut()),
    ] {
        assert!(result
            .expect_err("guest route blocked during preparation")
            .contains("preparing"));
    }
    assert!(app.world().resource::<Runtime>().connection.is_none());
    assert!(!app.world().contains_resource::<Hosted>());
    start::close(app.world_mut());
    start::finish_host(app.world_mut());
    assert!(
        !app.world().contains_resource::<Hosted>(),
        "cancelled receiver cannot resurrect host"
    );
}

#[test]
fn accepted_guest_transition_stops_browser_without_waiting_for_admission() {
    let mut app = minimal_app();
    let reserved = std::net::UdpSocket::bind("127.0.0.1:0").expect("isolated local endpoint");
    let port = reserved.local_addr().expect("reserved port").port();
    let prepared = PreparedDirectHost::new(
        DirectEndpoint::new("127.0.0.1", port).expect("valid endpoint"),
        SessionId::from_bytes([19; 16]),
        InviteToken::from_bytes([23; 16]),
    )
    .expect("test certificate; server is never opened");
    let encoded = prepared.connection_code().encode();
    discovery::browse_fake(app.world_mut());
    start::join_code(app.world_mut(), encoded.expose_for_sharing())
        .expect("begin pinned guest route");
    assert!(!app.world().contains_resource::<discovery::Browser>());
    assert!(
        !app.world().resource::<Runtime>().admitted,
        "browser stops at join intent, not only after welcome"
    );
    start::disconnect_guest(app.world_mut());
}

#[test]
fn closing_physical_endpoint_blocks_guest_routes_until_cleanup_finishes() {
    let mut app = minimal_app();
    let old = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<Runtime>().pending_close = Some((old, Instant::now()));
    assert!(start::join_code(app.world_mut(), "not-a-code")
        .expect_err("old listener closing")
        .contains("closing"));
    app.world_mut().resource_mut::<Runtime>().pending_close = None;
    app.world_mut().resource_mut::<Runtime>().pending_disconnect = Some((old, Instant::now()));
    assert!(start::reconnect(app.world_mut())
        .expect_err("old guest closing")
        .contains("closing"));
}

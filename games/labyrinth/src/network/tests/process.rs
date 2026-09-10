//! Explicit OS-process smoke gate. Six independent native processes communicate
//! over pinned UDP; only private fixture files hand invitations to each guest.

use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use super::*;

const CHILD_TEST: &str = "network::tests::process::process_child_entry";
const ROLE_ENV: &str = "LABYRINTH_PROCESS_TEST_ROLE";
const DIRECTORY_ENV: &str = "LABYRINTH_PROCESS_TEST_DIRECTORY";
const ROLES: [&str; TEST_PLAYERS] = [
    "host", "guest-a", "guest-b", "guest-c", "guest-d", "guest-e",
];
const RESTARTED_ROLE: &str = "guest-e";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Report {
    process: u32,
    admitted: bool,
    slot: Option<u8>,
    peer: Option<PeerId>,
    snapshot: Option<SessionSnapshot>,
}

fn report_path(directory: &Path, role: &str) -> PathBuf {
    directory.join(format!("{role}.report.json"))
}

fn read_report(directory: &Path, role: &str) -> Option<Report> {
    let bytes = fs::read(report_path(directory, role)).ok()?;
    if bytes.len() > 256 * 1024 {
        return None;
    }
    // Writers may be between truncate and write; incomplete public reports retry.
    serde_json::from_slice(&bytes).ok()
}

fn required_report(directory: &Path, role: &str) -> Report {
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut report = None;
    while Instant::now() < deadline {
        report = read_report(directory, role);
        if report.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    report.expect("completed public process report")
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).expect("private fixture file opens");
    file.write_all(bytes).expect("private fixture file writes");
}

fn child_command(directory: &Path, role: &str) -> Child {
    Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(ROLE_ENV, role)
        .env(DIRECTORY_ENV, directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("independent native test process starts")
}

struct Processes(Vec<Child>);

impl Processes {
    fn stop(&mut self) {
        for child in &mut self.0 {
            let _killed = child.kill();
            let _reaped = child.wait();
        }
    }

    fn wait(&mut self, timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self
                .0
                .iter_mut()
                .any(|child| child.try_wait().ok().flatten().is_some())
            {
                return false;
            }
            if condition() {
                return true;
            }
            thread::sleep(Duration::from_millis(15));
        }
        false
    }

    fn require(&mut self, timeout: Duration, condition: impl FnMut() -> bool, stage: &str) {
        let succeeded = self.wait(timeout, condition);
        if !succeeded {
            self.stop();
        }
        assert!(succeeded, "six-process smoke failed at {stage}");
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Only the parent smoke test sets these non-secret fixture environment variables.
#[test]
fn process_child_entry() {
    let Ok(role) = std::env::var(ROLE_ENV) else {
        return;
    };
    assert!(ROLES.contains(&role.as_str()), "invalid fixture role");
    let directory =
        PathBuf::from(std::env::var_os(DIRECTORY_ENV).expect("private fixture directory"));
    let options = crate::profile::LaunchOptions {
        profile: role.clone(),
        data_dir: Some(directory.clone()),
        ..crate::profile::LaunchOptions::default()
    };
    let profile = crate::profile::ProfileGuard::begin(&options).expect("exclusive process profile");
    let mut apps = vec![socket_app(Some(&profile.credential_path()))];
    if role == "host" {
        open_host(&mut apps, "");
        for (index, guest) in ROLES.iter().skip(1).enumerate() {
            let code =
                hosted_code(app(&mut apps, 0).world(), index).expect("independent invitation");
            write_private(&directory.join(format!("{guest}.invite")), code.as_bytes());
        }
    } else if profile.credential_path().exists() {
        start::reconnect(app(&mut apps, 0).world_mut())
            .expect("restarted process reads its own profile");
    } else {
        let path = directory.join(format!("{role}.invite"));
        let deadline = Instant::now() + Duration::from_secs(10);
        while !path.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let code =
            zeroize::Zeroizing::new(fs::read_to_string(path).expect("private invitation handoff"));
        start::join_code(app(&mut apps, 0).world_mut(), &code)
            .expect("child begins pinned transport");
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut ready_sent = false;
    let mut class_sent = false;
    let mut started = false;
    let mut acted_turn = 0;
    let mut reported = Instant::now();
    while Instant::now() < deadline && !directory.join("stop").exists() {
        let child = app(&mut apps, 0);
        child.update();
        let runtime = child.world().resource::<Runtime>();
        let latest = runtime.latest.clone();
        let admitted = runtime.admitted;
        let slot = runtime.player;
        if admitted {
            if let Some(snapshot) = latest.as_ref() {
                if snapshot.combat.is_none() && !ready_sent {
                    let chosen = snapshot
                        .players
                        .iter()
                        .find(|player| Some(player.slot) == slot);
                    if role == RESTARTED_ROLE
                        && chosen.is_some_and(|player| player.hero != HeroClass::Knifehand)
                    {
                        if !class_sent {
                            child
                                .world_mut()
                                .write_message(LabyrinthIntent::SelectHero(HeroClass::Knifehand));
                            class_sent = true;
                        }
                    } else if directory.join("party-ready").exists() {
                        child
                            .world_mut()
                            .write_message(LabyrinthIntent::Ready(true));
                        ready_sent = true;
                    }
                }
                if role == "host"
                    && !started
                    && snapshot
                        .players
                        .iter()
                        .all(|player| player.connected && player.ready)
                {
                    child
                        .world_mut()
                        .write_message(LabyrinthIntent::StartEncounter);
                    started = true;
                }
                if let Some(combat) = snapshot.combat.as_ref() {
                    if role == "host"
                        && !directory.join("resume").exists()
                        && combat
                            .active_actor
                            .and_then(|id| combat.actor(id))
                            .is_some_and(|actor| actor.team() == Team::Heroes)
                        && combat.actors.iter().any(|actor| {
                            actor
                                .statuses
                                .iter()
                                .any(|status| status.kind == StatusKind::Bleed)
                        })
                    {
                        write_private(
                            &directory.join("frozen"),
                            b"controllers paused at live bleed boundary",
                        );
                    }
                    let play =
                        !directory.join("frozen").exists() || directory.join("resume").exists();
                    if play && !snapshot.paused && combat.turn_id != acted_turn {
                        if let Some(actor) = combat.active_actor.filter(|actor| {
                            snapshot
                                .players
                                .iter()
                                .any(|player| Some(player.slot) == slot && player.actor == *actor)
                        }) {
                            child.world_mut().write_message(LabyrinthIntent::Combat {
                                actor,
                                action: aggressive_action(combat, actor),
                                encounter: snapshot.encounter,
                                decision: combat.turn_id,
                            });
                            acted_turn = combat.turn_id;
                        }
                    }
                }
            }
        }
        if reported.elapsed() >= Duration::from_millis(25) {
            let peer = child
                .world()
                .resource::<ReconnectCredentialStorage>()
                .store()
                .load()
                .ok()
                .flatten()
                .map(|record| record.peer_id);
            let report = Report {
                process: std::process::id(),
                admitted,
                slot,
                peer,
                snapshot: latest,
            };
            let bytes = serde_json::to_vec(&report).expect("public report serializes");
            write_private(&report_path(&directory, &role), &bytes);
            reported = Instant::now();
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "Explicit native six-process restart gate; run with --ignored --exact"]
fn six_native_processes_survive_guest_kill_and_finish_the_fight() {
    let directory = tempfile::tempdir().expect("private process fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
            .expect("private fixture directory permissions");
    }
    let root = directory.path();
    let mut processes = Processes(vec![child_command(root, "host")]);
    processes.require(
        Duration::from_secs(10),
        || root.join("guest-e.invite").exists(),
        "host invitation setup",
    );
    for role in ROLES.iter().skip(1) {
        processes.0.push(child_command(root, role));
        processes.require(
            Duration::from_secs(10),
            || read_report(root, role).is_some_and(|report| report.admitted),
            "sequential guest admission establishes the sixth-seat restart fixture",
        );
    }
    processes.require(
        Duration::from_secs(5),
        || {
            read_report(root, RESTARTED_ROLE)
                .and_then(|report| report.snapshot)
                .is_some_and(|snapshot| {
                    snapshot
                        .players
                        .iter()
                        .any(|player| player.slot == 5 && player.hero == HeroClass::Knifehand)
                })
        },
        "sixth player selects a repeated class before readiness",
    );
    write_private(
        &root.join("party-ready"),
        b"all six controllers may ready now",
    );
    let mut last_combat = None;
    let mut stable_since = Instant::now();
    processes.require(
        Duration::from_secs(15),
        || {
            if !root.join("frozen").exists() {
                return false;
            }
            let Some(report) = read_report(root, "host") else {
                return false;
            };
            let Some(snapshot) = report.snapshot else {
                return false;
            };
            if snapshot.combat != last_combat {
                last_combat = snapshot.combat.clone();
                stable_since = Instant::now();
            }
            snapshot.combat.is_some()
                && !snapshot.paused
                && stable_since.elapsed() >= Duration::from_millis(250)
                && ROLES.iter().skip(1).all(|role| {
                    read_report(root, role).is_some_and(|guest| {
                        guest.admitted
                            && guest
                                .snapshot
                                .as_ref()
                                .is_some_and(|guest| guest.combat == snapshot.combat)
                    })
                })
        },
        "six admitted processes at a stable live-bleed boundary",
    );
    let before_session = required_report(root, "host")
        .snapshot
        .expect("stable session");
    assert_eq!(before_session.players.len(), TEST_PLAYERS);
    assert!(before_session
        .players
        .iter()
        .all(|player| player.connected && player.ready));
    let actor_ids: BTreeSet<_> = before_session
        .players
        .iter()
        .map(|player| player.actor)
        .collect();
    assert_eq!(actor_ids.len(), TEST_PLAYERS);
    let original = required_report(root, RESTARTED_ROLE);
    assert_eq!(original.slot, Some(5), "kill the actual sixth player");
    let owned = before_session
        .players
        .iter()
        .find(|player| Some(player.slot) == original.slot)
        .expect("late seat owns a hero")
        .clone();
    assert_eq!(owned.hero, HeroClass::Knifehand);
    assert!(before_session
        .players
        .iter()
        .any(|player| player.actor != owned.actor && player.hero == owned.hero));
    let before = before_session.combat.expect("stable combat");
    let owned_actor = before
        .actor(owned.actor)
        .expect("sixth combat actor")
        .clone();
    assert_eq!(owned_actor.abilities, owned.abilities);
    assert!(before.actors.iter().any(|actor| actor
        .statuses
        .iter()
        .any(|status| status.kind == StatusKind::Bleed)));
    let mut killed = processes.0.remove(LAST_GUEST);
    assert_eq!(killed.id(), original.process);
    killed
        .kill()
        .expect("terminate guest process without cleanup or Leave");
    killed.wait().expect("reap killed guest");
    // The reaped handle is no longer part of the remaining-process liveness check.
    processes.require(
        Duration::from_secs(10),
        || {
            read_report(root, "host")
                .is_some_and(|report| report.snapshot.is_some_and(|snapshot| snapshot.paused))
        },
        "host observes actual process death",
    );
    let paused = required_report(root, "host")
        .snapshot
        .expect("paused snapshot");
    assert_eq!(
        paused.combat.as_ref(),
        Some(&before),
        "guest process loss advanced bleed/initiative"
    );
    let replacement = child_command(root, RESTARTED_ROLE);
    let replacement_id = replacement.id();
    processes.0.insert(LAST_GUEST, replacement);
    processes.require(
        Duration::from_secs(10),
        || {
            let Some(host) = read_report(root, "host").and_then(|report| report.snapshot) else {
                return false;
            };
            let Some(guest) = read_report(root, RESTARTED_ROLE) else {
                return false;
            };
            !host.paused
                && host.players.iter().all(|player| player.connected)
                && guest.process == replacement_id
                && guest.admitted
                && guest.peer == original.peer
                && guest.slot == original.slot
                && guest
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.combat == paused.combat)
        },
        "new guest process restores profile identity and exact combat state",
    );
    let recovered = required_report(root, RESTARTED_ROLE)
        .snapshot
        .expect("recovered guest snapshot");
    assert_eq!(
        recovered
            .players
            .iter()
            .find(|player| player.slot == owned.slot),
        Some(&owned),
        "sixth process restart changed the reserved actor, class or loadout"
    );
    assert_eq!(
        recovered
            .combat
            .as_ref()
            .and_then(|combat| combat.actor(owned.actor)),
        Some(&owned_actor),
        "sixth process restart changed its HP, loadout or status instances"
    );
    write_private(&root.join("resume"), b"resume synthetic controllers");
    processes.require(
        Duration::from_secs(30),
        || {
            let Some(host) = read_report(root, "host")
                .and_then(|report| report.snapshot)
                .and_then(|snapshot| snapshot.combat)
            else {
                return false;
            };
            host.outcome.is_some()
                && host.revision > before.revision
                && ROLES.iter().skip(1).all(|role| {
                    read_report(root, role)
                        .and_then(|report| report.snapshot)
                        .and_then(|snapshot| snapshot.combat)
                        .is_some_and(|guest| guest == host)
                })
        },
        "six processes finish the fight after restart",
    );
    write_private(&root.join("stop"), b"stop fixture");
    processes.stop();
}

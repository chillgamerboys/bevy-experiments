//! Actual compiled native protocol peers and byte-bound cache checks.
#[cfg(unix)]
mod posix {
    use gameskills_cli::installation::{activate_codex, execute, native_argv};
    use serde_json::{json, Value};
    use std::{
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
    static NATIVE_PROCESS_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct Fixture {
        _temp: tempfile::TempDir,
        root: PathBuf,
        bundle: PathBuf,
        executable: PathBuf,
        scenario: Value,
    }
    fn copy(source: &Path, target: &Path) -> Result {
        fs::create_dir_all(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                copy(&entry.path(), &target.join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), target.join(entry.file_name()))?;
            }
        }
        Ok(())
    }
    impl Fixture {
        fn new() -> Result<Self> {
            let temp = tempfile::Builder::new()
                .prefix("gameskills native fixture ")
                .tempdir()?;
            let root = temp.path().canonicalize()?;
            let bundle = root.join("bundle");
            let home = root.join("native-home");
            fs::create_dir(&home)?;
            let args = [
                "--out",
                "bundle",
                "--packages",
                "gameskills",
                "gameskills-ui",
            ]
            .map(OsString::from);
            let out = execute(&root, "bundle", &args)?;
            let market = at(&out, "/marketplace").as_str().ok_or("market")?;
            let catalog: Value =
                serde_json::from_slice(&fs::read(bundle.join("plugins/gameskills/catalog.json"))?)?;
            let mut skills = Vec::new();
            let mut plugins = Vec::new();
            for package in ["gameskills", "gameskills-ui"] {
                let cache = home
                    .join("plugins/cache")
                    .join(market)
                    .join(package)
                    .join("0.1.0");
                copy(&bundle.join("plugins").join(package), &cache)?;
                for name in at(&catalog, &format!("/packages/{package}/skills"))
                    .as_array()
                    .ok_or("skills")?
                {
                    let name = name.as_str().ok_or("name")?;
                    skills.push(json!({"name":format!("{package}:{name}"),"pluginId":format!("{package}@{market}"),"enabled":true,"path":cache.join("skills").join(name).join("SKILL.md")}));
                }
                plugins.push(
                    json!({"id":format!("{package}@{market}"),"enabled":true,"installed":true}),
                );
            }
            let scenario = json!({"home":home,"market":market,"skills":skills,"plugins":plugins,"mode":"success"});
            let profile = std::env::current_exe()?
                .parent()
                .and_then(Path::parent)
                .ok_or("test profile")?
                .to_owned();
            let probe = profile
                .join("examples")
                .join(format!("native_probe{}", std::env::consts::EXE_SUFFIX));
            if !probe.is_file() {
                return Err(format!("compiled probe missing at {}; build the native_probe example before focused native tests",probe.display()).into());
            }
            let executable = root.join("fake-codex");
            fs::copy(probe, &executable)?;
            Ok(Self {
                _temp: temp,
                root,
                bundle,
                executable,
                scenario,
            })
        }
        fn activate(&self, timeout: f64) -> std::result::Result<Value, String> {
            fs::write(
                self.root.join("scenario.json"),
                serde_json::to_vec(&self.scenario).expect("scenario JSON"),
            )
            .expect("write scenario");
            activate_codex(&self.bundle, &self.root, &self.executable, timeout)
        }
        fn dead(&self) -> Result {
            let pid: i32 = fs::read_to_string(self.root.join("pid"))?
                .parse()
                .map_err(|error| format!("invalid peer PID observation: {error}"))?;
            assert!(nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_err());
            Ok(())
        }
    }
    #[test]
    fn native_argv_binds_selected_packages_without_config_writes() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fixture = Fixture::new()?;
        let codex = native_argv(
            &fixture.bundle,
            "codex",
            &["exec".into(), "--ephemeral".into()],
        )?;
        let claude = native_argv(
            &fixture.bundle,
            "claude",
            &["--print".into(), "test".into()],
        )?;
        assert_eq!(claude.first().map(String::as_str), Some("claude"));
        assert_eq!(
            claude
                .iter()
                .filter(|s| s.as_str() == "--plugin-dir")
                .count(),
            2
        );
        assert!(codex.iter().any(|s| s.contains("plugins.gameskills@")));
        assert!(!fixture.root.join("gameskills.toml").exists());
        assert!(native_argv(&fixture.bundle, "other", &[]).is_err());
        let mut relative = PathBuf::new();
        for _ in std::env::current_dir()?.components().skip(1) {
            relative.push("..");
        }
        relative.push(fixture.bundle.strip_prefix("/")?);
        assert_eq!(
            native_argv(&relative, "claude", &["--print".into(), "test".into()])?,
            claude
        );

        Ok(())
    }
    #[test]
    fn actual_peer_observes_exact_catalog_with_no_model_or_install_rpc() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fixture = Fixture::new()?;
        let result = fixture.activate(2.0)?;
        assert_eq!(at(&result, "/skills").as_array().ok_or("skills")?.len(), 14);
        assert!(at(&result, "/claim")
            .as_str()
            .ok_or("claim")?
            .contains("behavior not tested"));
        let trace = fs::read_to_string(fixture.root.join("requests.jsonl"))?;
        let methods = trace
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line).map(|value| {
                    at(&value, "/method")
                        .as_str()
                        .unwrap_or_default()
                        .to_owned()
                })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(
            methods,
            [
                "initialize",
                "initialized",
                "plugin/list",
                "skills/list",
                "plugin/list"
            ]
        );
        fixture.dead()?;
        Ok(())
    }
    #[test]
    fn delayed_native_materialization_is_retried_within_one_deadline() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut fixture = Fixture::new()?;
        *fixture
            .scenario
            .pointer_mut("/mode")
            .expect("fixture field") = json!("delayed");
        fixture.activate(2.0)?;
        let trace = fs::read_to_string(fixture.root.join("requests.jsonl"))?;
        assert_eq!(trace.matches("skills/list").count(), 2);
        fixture.dead()?;
        Ok(())
    }
    #[test]
    fn missing_plugins_and_wrong_pins_fail_without_skill_claim() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut fixture = Fixture::new()?;
        fixture
            .scenario
            .pointer_mut("/plugins")
            .expect("fixture field")
            .as_array_mut()
            .ok_or("plugins")?
            .pop();
        assert!(fixture
            .activate(2.0)
            .expect_err("missing")
            .contains("missing selected"));
        fixture.dead()?;
        let mut fixture = Fixture::new()?;
        *fixture
            .scenario
            .pointer_mut("/skills/0/pluginId")
            .expect("fixture field") = json!("gameskills@wrong");
        assert!(fixture
            .activate(2.0)
            .expect_err("wrong pin")
            .contains("another GameSkills pin"));
        fixture.dead()?;
        Ok(())
    }
    #[test]
    fn complete_native_package_bytes_and_cache_location_are_verified() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fixture = Fixture::new()?;
        let path = at(&fixture.scenario, "/skills/0/path")
            .as_str()
            .ok_or("path")?;
        fs::write(path, "tampered")?;
        assert!(fixture
            .activate(2.0)
            .expect_err("tamper")
            .contains("cache content differs"));
        fixture.dead()?;
        let mut fixture = Fixture::new()?;
        *fixture
            .scenario
            .pointer_mut("/skills/0/path")
            .expect("fixture field") = json!(fixture
            .bundle
            .join("plugins/gameskills/skills/setup/SKILL.md"));
        assert!(fixture
            .activate(2.0)
            .expect_err("source not native")
            .contains("outside the pinned native cache"));
        fixture.dead()?;
        Ok(())
    }
    #[test]
    fn protocol_errors_are_bounded_and_reap_the_actual_child() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for mode in [
            "error",
            "malformed",
            "exit",
            "wrong_id",
            "interactive",
            "oversized",
        ] {
            let mut fixture = Fixture::new()?;
            *fixture
                .scenario
                .pointer_mut("/mode")
                .expect("fixture field") = json!(mode);
            let start = Instant::now();
            assert!(fixture.activate(2.0).is_err(), "{mode}");
            assert!(start.elapsed() < Duration::from_secs(4), "{mode}");
            fixture.dead()?;
        }
        Ok(())
    }
    #[test]
    fn missing_skills_and_unresponsive_peer_exhaust_deadline_and_cleanup() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for mode in ["missing", "hang", "child_pipe"] {
            let mut fixture = Fixture::new()?;
            if mode == "missing" {
                *fixture
                    .scenario
                    .pointer_mut("/skills")
                    .expect("fixture field") = json!([]);
            } else {
                *fixture
                    .scenario
                    .pointer_mut("/mode")
                    .expect("fixture field") = json!(mode);
            }
            let start = Instant::now();
            assert!(fixture.activate(1.2).is_err());
            assert!(start.elapsed() < Duration::from_secs(4));
            fixture.dead()?;
            if mode == "child_pipe" {
                let pid = fs::read_to_string(fixture.root.join("child-pid"))?;
                let output = std::process::Command::new("ps")
                    .args(["-o", "stat=", "-p", pid.trim()])
                    .output()?;
                let status = String::from_utf8_lossy(&output.stdout);
                assert!(
                    status.trim().is_empty() || status.trim().starts_with('Z'),
                    "{status}"
                );
            }
        }
        Ok(())
    }
    #[test]
    fn duplicates_disabled_plugins_and_invalid_timeout_do_not_pass() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut fixture = Fixture::new()?;
        let first = at(&fixture.scenario, "/skills/0").clone();
        fixture
            .scenario
            .pointer_mut("/skills")
            .expect("fixture field")
            .as_array_mut()
            .ok_or("skills")?
            .push(first);
        assert!(fixture
            .activate(1.0)
            .expect_err("duplicate")
            .contains("duplicate"));
        let mut fixture = Fixture::new()?;
        *fixture
            .scenario
            .pointer_mut("/plugins/0/enabled")
            .expect("fixture field") = json!(false);
        assert!(fixture
            .activate(1.0)
            .expect_err("disabled")
            .contains("enable"));
        for timeout in [0.0, -1.0, 121.0, f64::NAN, f64::INFINITY] {
            assert!(
                activate_codex(&fixture.bundle, &fixture.root, &fixture.executable, timeout)
                    .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn cli_launch_propagates_native_exit_and_respects_client_selection() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fixture = Fixture::new()?;
        execute(&fixture.root, "setup", &[OsString::from("--apply")])?;
        fs::copy(&fixture.executable, fixture.root.join("claude"))?;
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .args([
                "--root",
                fixture.root.to_str().ok_or("root")?,
                "native",
                "claude",
                "--launch",
                "--",
                "--client-exit",
                "7",
            ])
            .env("PATH", &fixture.root)
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(7),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value.get("user_configuration_writes"), Some(&json!(false)));
        fs::write(
            fixture.root.join("gameskills.toml"),
            "schema_version=1\nclients=[\"claude\"]\n",
        )?;
        assert!(execute(&fixture.root, "native", &[OsString::from("codex")])
            .expect_err("unselected client")
            .contains("not selected"));
        Ok(())
    }
    #[test]
    fn incomplete_discovery_keeps_one_deadline_and_reaps_the_actual_peer() -> Result {
        let _guard = NATIVE_PROCESS_TEST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut fixture = Fixture::new()?;
        *fixture.scenario.get_mut("mode").ok_or("mode")? = json!("backpressure");
        fs::write(
            fixture.root.join("scenario.json"),
            serde_json::to_vec(&fixture.scenario)?,
        )?;
        // This exercises the complete protocol loop with incomplete responses.
        // The private Rpc regression deterministically fills stdin in one write;
        // no retry count or host-specific pipe capacity is assumed here.
        let cwd = fixture.root.clone();
        let bundle = fixture.bundle.clone();
        let executable = fixture.executable.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let started = Instant::now();
        let worker = std::thread::spawn(move || {
            let result = activate_codex(&bundle, &cwd, &executable, 1.5);
            let _ = sender.send(result);
        });
        let observed = receiver.recv_timeout(Duration::from_secs(5));
        // A regression must fail the test, not strand Cargo on a blocked writer.
        if observed.is_err() {
            if let Ok(pid) = fs::read_to_string(fixture.root.join("pid")) {
                let pid = nix::unistd::Pid::from_raw(pid.trim().parse()?);
                let _ = nix::sys::signal::killpg(pid, nix::sys::signal::Signal::SIGKILL);
            }
        }
        worker.join().expect("native activation worker");
        let error = observed
            .map_err(|error| format!("activation exceeded bounded test wait: {error}"))?
            .expect_err("incomplete discovery must exhaust the shared deadline");
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(4));
        assert_eq!(
            fs::read_to_string(fixture.root.join("stopped-draining"))?,
            "true"
        );
        let sent: u64 = fs::read_to_string(fixture.root.join("response-count"))?
            .parse()
            .map_err(|error| format!("invalid response count observation: {error}"))?;
        assert!(
            sent >= 4,
            "probe did not exercise repeated incomplete discovery"
        );
        fixture.dead()?;
        Ok(())
    }
    fn at<'a>(value: &'a Value, path: &str) -> &'a Value {
        value.pointer(path).expect("fixture field")
    }
}

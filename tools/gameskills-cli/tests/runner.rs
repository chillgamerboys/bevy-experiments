//! Real Git and compiled subprocess regression contracts for the Rust runner.

#[cfg(not(unix))]
#[test]
fn windows_is_explicitly_unsupported() {
    let error = gameskills_cli::runner::execute(
        std::path::Path::new("."),
        &serde_json::json!({}),
        "run",
        &[],
    )
    .err();
    assert!(error.is_some_and(|e| e.contains("unsupported")));
}

#[cfg(unix)]
mod unix {
    use serde_json::{json, Value};
    use std::error::Error;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};
    type Test = Result<(), Box<dyn Error>>;
    struct Fixture {
        scratch: tempfile::TempDir,
        root: PathBuf,
        config: Value,
        probe: PathBuf,
    }
    struct ChildGuard(Option<Child>);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(child) = &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    impl ChildGuard {
        fn pid(&self) -> Result<u32, Box<dyn Error>> {
            Ok(self.0.as_ref().ok_or("no child")?.id())
        }
        fn collect(&mut self) -> Result<(i32, Value), Box<dyn Error>> {
            let start = Instant::now();
            loop {
                let child = self.0.as_mut().ok_or("no child")?;
                if child.try_wait()?.is_some() {
                    break;
                }
                if start.elapsed() > Duration::from_secs(15) {
                    return Err("runner fixture exceeded deadline".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let output = self.0.take().ok_or("no child")?.wait_with_output()?;
            let code = output.status.code().unwrap_or(-1);
            let value = if output.stdout.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&output.stdout).map_err(|e| {
                    format!(
                        "{e}: {} / {}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    )
                })?
            };
            Ok((code, value))
        }
        fn signal(&self, signal: nix::sys::signal::Signal) -> Result<(), Box<dyn Error>> {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(i32::try_from(self.pid()?)?),
                signal,
            )?;
            Ok(())
        }
    }
    impl Fixture {
        fn new() -> Result<Self, Box<dyn Error>> {
            let scratch = tempfile::Builder::new()
                .prefix("Rust runner spaces ")
                .tempdir()?;
            let root = scratch.path().join("project");
            fs::create_dir(&root)?;
            let root = root.canonicalize()?;
            let current = std::env::current_exe()?;
            let probe = current
                .parent()
                .and_then(Path::parent)
                .ok_or("no target profile")?
                .join("examples/runner_probe");
            assert!(
                probe.is_file(),
                "build runner_probe example first: {}",
                probe.display()
            );
            let mut fixture = Self {
                scratch,
                root,
                config: json!({"commands":{},"dispatch":{"max_workers":5}}),
                probe,
            };
            fixture.git(&["init", "--quiet"])?;
            fs::write(fixture.root.join(".gitignore"), ".gameskills/\n")?;
            fs::write(fixture.root.join("source.txt"), "original\n")?;
            fixture.commit("initial")?;
            gameskills_cli::installation::execute(&fixture.root, "setup", &["--apply".into()])?;
            fixture.config = gameskills_cli::installation::ready_config(&fixture.root)?;
            fixture
                .config
                .as_object_mut()
                .ok_or("config")?
                .insert("commands".into(), json!({}));
            fixture.sync_config()?;
            Ok(fixture)
        }
        fn sync_config(&self) -> Result<(), Box<dyn Error>> {
            fn convert(value: &Value) -> Result<toml_edit::Value, Box<dyn Error>> {
                Ok(match value {
                    Value::String(value) => value.as_str().into(),
                    Value::Bool(value) => (*value).into(),
                    Value::Number(value) => {
                        if let Some(value) = value.as_i64() {
                            value.into()
                        } else {
                            value.as_f64().ok_or("number")?.into()
                        }
                    }
                    Value::Array(values) => {
                        let mut array = toml_edit::Array::new();
                        for value in values {
                            array.push(convert(value)?);
                        }
                        array.into()
                    }
                    Value::Object(values) => {
                        let mut table = toml_edit::InlineTable::new();
                        for (key, value) in values {
                            table.insert(key, convert(value)?);
                        }
                        table.into()
                    }
                    Value::Null => return Err("fixture TOML cannot represent null".into()),
                })
            }
            let mut document = toml_edit::DocumentMut::new();
            for (key, value) in self.config.as_object().ok_or("config")? {
                document.insert(key, toml_edit::Item::Value(convert(value)?));
            }
            fs::write(self.root.join("gameskills.toml"), document.to_string())?;
            Ok(())
        }
        fn git(&self, args: &[&str]) -> Result<String, Box<dyn Error>> {
            let output = Command::new("git")
                .args(args)
                .current_dir(&self.root)
                .output()?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
            }
            Ok(String::from_utf8(output.stdout)?.trim().into())
        }
        fn commit(&self, message: &str) -> Result<(), Box<dyn Error>> {
            self.git(&["add", "."])?;
            self.git(&[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--quiet",
                "-m",
                message,
            ])?;
            Ok(())
        }
        fn command(
            &mut self,
            name: &str,
            args: &[&str],
            options: Value,
        ) -> Result<(), Box<dyn Error>> {
            let mut argv = vec![self.probe.to_string_lossy().into_owned()];
            argv.extend(args.iter().map(|s| s.to_string()));
            let mut spec = options.as_object().ok_or("invalid options")?.clone();
            spec.insert("argv".into(), json!(argv));
            self.config
                .get_mut("commands")
                .and_then(Value::as_object_mut)
                .ok_or("missing commands")?
                .insert(name.into(), Value::Object(spec));
            self.sync_config()?;
            Ok(())
        }
        fn command_raw(&mut self, name: &str, spec: Value) -> Result<(), Box<dyn Error>> {
            self.config
                .get_mut("commands")
                .and_then(Value::as_object_mut)
                .ok_or("no commands")?
                .insert(name.into(), spec);
            self.sync_config()?;
            Ok(())
        }
        fn prepared(&self, family: &str, args: &[&str]) -> Result<Command, Box<dyn Error>> {
            let file = tempfile::NamedTempFile::new_in(self.scratch.path())?;
            fs::write(file.path(), serde_json::to_vec(&self.config)?)?;
            let (_, path) = file.keep()?;
            let mut command = Command::new(&self.probe);
            command
                .arg("harness")
                .arg(&self.root)
                .arg(path)
                .arg(family)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            Ok(command)
        }
        fn launch(&self, family: &str, args: &[&str]) -> Result<ChildGuard, Box<dyn Error>> {
            Ok(ChildGuard(Some(self.prepared(family, args)?.spawn()?)))
        }
        fn invoke(&self, family: &str, args: &[&str]) -> Result<(i32, Value), Box<dyn Error>> {
            self.launch(family, args)?.collect()
        }
        fn run(&self, args: &[&str]) -> Result<Value, Box<dyn Error>> {
            let (code, value) = self.invoke("run", args)?;
            assert_ne!(code, 2, "{value}");
            Ok(value)
        }
        fn evidence(&self, id: &str, operation: &str) -> Result<Value, Box<dyn Error>> {
            let (_, value) = self.invoke("evidence", &[operation, id])?;
            Ok(value)
        }
        fn id<'a>(&self, value: &'a Value) -> Result<&'a str, Box<dyn Error>> {
            value
                .get("run_id")
                .and_then(Value::as_str)
                .ok_or("no run id".into())
        }
        fn record(&self, id: &str) -> PathBuf {
            self.root
                .join(".gameskills/runs")
                .join(id)
                .join("record.json")
        }
        fn log(&self, value: &Value, name: &str, stream: &str) -> Result<String, Box<dyn Error>> {
            Ok(fs::read_to_string(
                self.record(self.id(value)?)
                    .parent()
                    .ok_or("no run dir")?
                    .join(format!("{name}.{stream}.log")),
            )?)
        }
        fn ready(&self, path: &Path) -> Result<(), Box<dyn Error>> {
            let deadline = Instant::now() + Duration::from_secs(8);
            while !path.exists() {
                if Instant::now() > deadline {
                    return Err(format!("fixture did not create {}", path.display()).into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }
    }
    fn ok(value: &Value) -> bool {
        value.get("ok") == Some(&json!(true))
    }
    fn status<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
        value
            .get("results")
            .and_then(|r| r.get(name))
            .and_then(|r| r.get("status"))
            .and_then(Value::as_str)
    }

    #[test]
    fn evidence_list_is_read_only_and_missing_setup_does_not_create_state() -> Test {
        let f = Fixture::new()?;
        fs::remove_dir_all(f.root.join(".gameskills"))?;
        fs::remove_file(f.root.join("gameskills.toml"))?;
        fs::remove_file(f.root.join("gameskills.lock.json"))?;
        let (code, result) = f.invoke("evidence", &["list"])?;
        assert_eq!(code, 0);
        assert_eq!(result.get("runs"), Some(&json!([])));
        assert!(!f.root.join(".gameskills").exists());
        Ok(())
    }
    #[test]
    fn deterministic_dag_runs_shared_dependency_once_and_captures_output() -> Test {
        let mut f = Fixture::new()?;
        f.command("base", &["echo", "base"], json!({}))?;
        f.command("zeta", &["echo", "zeta"], json!({"requires":["base"]}))?;
        f.command("alpha", &["echo", "alpha"], json!({"requires":["base"]}))?;
        let result = f.run(&["zeta", "alpha", "alpha"])?;
        assert!(ok(&result), "{result}");
        let shown = f.evidence(f.id(&result)?, "show")?;
        assert!(ok(&shown), "{shown}");
        assert_eq!(
            shown.pointer("/record/order"),
            Some(&json!(["base", "alpha", "zeta"]))
        );
        assert_eq!(shown.pointer("/record/runtime"), Some(&json!("rust")));
        assert_eq!(shown.pointer("/record/schema_version"), Some(&json!(2)));
        assert_eq!(f.log(&result, "alpha", "stderr")?, "diagnostic\n");
        Ok(())
    }
    #[test]
    fn arguments_are_not_interpreted_by_a_shell() -> Test {
        let mut f = Fixture::new()?;
        let literal = "$(touch SHELL_EXPANDED); * > redirect";
        f.command("literal", &["echo", literal], json!({}))?;
        let result = f.run(&["literal"])?;
        assert!(ok(&result), "{result}");
        assert_eq!(f.log(&result, "literal", "stdout")?, format!("{literal}\n"));
        assert!(!f.root.join("SHELL_EXPANDED").exists());
        Ok(())
    }
    #[test]
    fn failure_preserves_logs_and_skips_dependents_but_runs_independent_work() -> Test {
        let mut f = Fixture::new()?;
        f.command("fail", &["fail"], json!({}))?;
        f.command(
            "dependent",
            &["echo", "must not execute"],
            json!({"requires":["fail"]}),
        )?;
        f.command("independent", &["echo", "useful"], json!({}))?;
        let result = f.run(&["dependent", "independent"])?;
        assert!(!ok(&result));
        assert_eq!(status(&result, "dependent"), Some("skipped"));
        assert_eq!(status(&result, "independent"), Some("passed"));
        assert_eq!(result.pointer("/results/fail/exit_code"), Some(&json!(7)));
        assert_eq!(f.log(&result, "fail", "stderr")?, "why\n");
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        Ok(())
    }
    #[test]
    fn invalid_graphs_and_traversal_are_rejected_before_writing() -> Test {
        let mut f = Fixture::new()?;
        for spec in [
            json!({"argv":[]}),
            json!({"argv":"echo untrusted"}),
            json!({"argv":["echo"],"cwd":"../outside"}),
            json!({"argv":["echo"],"cwd":"C:\\outside"}),
            json!({"argv":["echo"],"requires":["missing"]}),
            json!({"argv":["echo"],"requires":["a"]}),
            json!({"argv":["echo"],"timeout_seconds":true}),
            json!({"argv":["echo"],"requires":["b","b"]}),
            json!({"argv":["echo"],"cwd":".gameskills"}),
        ] {
            f.command_raw("a", spec)?;
            assert_eq!(f.invoke("run", &["a"])?.0, 2);
            assert!(!f.root.join(".gameskills/runs").exists());
        }
        Ok(())
    }
    #[test]
    fn cwd_symlink_is_rejected_and_real_subdirectory_is_used() -> Test {
        let mut f = Fixture::new()?;
        fs::create_dir(f.root.join("nested"))?;
        symlink("nested", f.root.join("alias"))?;
        f.command("a", &["cwd"], json!({"cwd":"alias"}))?;
        assert_eq!(f.invoke("run", &["a"])?.0, 2);
        f.command("a", &["cwd"], json!({"cwd":"nested"}))?;
        let result = f.run(&["a"])?;
        assert!(ok(&result), "{result}");
        assert_eq!(
            f.log(&result, "a", "stdout")?.trim(),
            f.root.join("nested").to_string_lossy()
        );
        Ok(())
    }

    fn descendant_case(success: bool, signal: Option<nix::sys::signal::Signal>) -> Test {
        let mut f = Fixture::new()?;
        let ready = f.root.join(".gameskills/ready");
        let escaped = f.root.join(".gameskills/escaped");
        f.command(
            "hang",
            &[
                "spawn",
                ready.to_str().ok_or("path")?,
                escaped.to_str().ok_or("path")?,
                if success { "exit" } else { "hang" },
            ],
            if signal.is_none() && !success {
                json!({"timeout_seconds":0.2})
            } else {
                json!({})
            },
        )?;
        f.command("later", &["echo", "later"], json!({"requires":["hang"]}))?;
        let result = if let Some(signal) = signal {
            let mut child = f.launch("run", &["later"])?;
            f.ready(&ready)?;
            child.signal(signal)?;
            let (_, result) = child.collect()?;
            assert_eq!(result.get("status"), Some(&json!("interrupted")));
            assert_eq!(status(&result, "later"), Some("skipped"));
            result
        } else {
            f.run(&["hang"])?
        };
        assert_eq!(
            status(&result, "hang"),
            Some(if signal.is_some() {
                "interrupted"
            } else if success {
                "passed"
            } else {
                "timeout"
            }),
            "{result}"
        );
        assert!(f.log(&result, "hang", "stdout")?.contains("started"));
        std::thread::sleep(Duration::from_millis(1250));
        assert!(!escaped.exists());
        Ok(())
    }
    #[test]
    fn timeout_kills_descendant_and_preserves_partial_output() -> Test {
        descendant_case(false, None)
    }
    #[test]
    fn successful_leader_cannot_leave_background_descendants() -> Test {
        descendant_case(true, None)
    }
    #[test]
    fn sigterm_cleans_descendants_records_interruption_and_skips_pending() -> Test {
        descendant_case(false, Some(nix::sys::signal::Signal::SIGTERM))
    }
    fn interval(f: &Fixture, result: &Value, name: &str) -> Result<(f64, f64), Box<dyn Error>> {
        let log = f.log(result, name, "stdout")?;
        let mut lines = log.lines();
        Ok((
            lines.next().ok_or("no start")?.parse()?,
            lines.next().ok_or("no finish")?.parse()?,
        ))
    }
    #[test]
    fn concurrency_is_useful_and_shared_resources_do_not_overlap() -> Test {
        let mut f = Fixture::new()?;
        for name in ["a", "b"] {
            f.command(name, &["interval", "250"], json!({}))?;
        }
        let concurrent = f.run(&["a", "b", "--max-workers", "2"])?;
        assert!(ok(&concurrent), "{concurrent}");
        let (a, b) = (
            interval(&f, &concurrent, "a")?,
            interval(&f, &concurrent, "b")?,
        );
        assert!(a.0.max(b.0) < a.1.min(b.1));
        let resource = format!("test:{}", f.root.display());
        for name in ["a", "b"] {
            f.command(name, &["interval", "250"], json!({"resources":[resource]}))?;
        }
        let serial = f.run(&["a", "b", "--max-workers", "2"])?;
        let (a, b) = (interval(&f, &serial, "a")?, interval(&f, &serial, "b")?);
        assert!(b.0 >= a.1);
        Ok(())
    }
    #[test]
    fn resource_locks_coordinate_separate_runner_processes() -> Test {
        let mut f = Fixture::new()?;
        f.command(
            "work",
            &["interval", "250"],
            json!({"resources":[format!("test:{}",f.root.display())]}),
        )?;
        let (mut a, mut b) = (f.launch("run", &["work"])?, f.launch("run", &["work"])?);
        let (_, a) = a.collect()?;
        let (_, b) = b.collect()?;
        assert!(ok(&a) && ok(&b), "{a} {b}");
        let (mut a, mut b) = (interval(&f, &a, "work")?, interval(&f, &b, "work")?);
        if a.0 > b.0 {
            std::mem::swap(&mut a, &mut b);
        }
        assert!(b.0 >= a.1);
        Ok(())
    }
    #[test]
    fn resource_wait_expiry_is_skipped_and_does_not_block_unrelated_work() -> Test {
        let mut f = Fixture::new()?;
        let resource = format!("test:{}", f.root.display());
        let ready = f.root.join(".gameskills/ready");
        let escaped = f.root.join(".gameskills/escaped");
        f.command(
            "hold",
            &[
                "spawn",
                ready.to_str().ok_or("path")?,
                escaped.to_str().ok_or("path")?,
                "hang",
            ],
            json!({"resources":[resource]}),
        )?;
        f.command("blocked", &["echo", "no"], json!({"resources":[resource]}))?;
        f.command("useful", &["echo", "yes"], json!({}))?;
        let mut holder = f.launch("run", &["hold"])?;
        f.ready(&ready)?;
        let result = f.run(&["blocked", "useful", "--resource-wait-seconds", "0"])?;
        assert_eq!(status(&result, "blocked"), Some("skipped"));
        assert_eq!(status(&result, "useful"), Some("passed"));
        holder.signal(nix::sys::signal::Signal::SIGTERM)?;
        holder.collect()?;
        Ok(())
    }
    #[test]
    fn configured_worker_cap_is_enforced() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "a"], json!({}))?;
        *f.config
            .pointer_mut("/dispatch/max_workers")
            .ok_or("no cap")? = json!(1);
        for count in ["0", "2"] {
            assert_eq!(f.invoke("run", &["a", "--max-workers", count])?.0, 2);
        }
        Ok(())
    }

    #[test]
    fn changed_worktree_untracked_index_head_config_lock_and_environment_invalidate() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let result = f.run(&["a"])?;
        assert!(ok(&result), "{result}");
        let id = f.id(&result)?;
        fs::write(f.root.join("source.txt"), "changed")?;
        assert!(!ok(&f.evidence(id, "validate")?));
        fs::write(f.root.join("source.txt"), "original\n")?;
        assert!(ok(&f.evidence(id, "validate")?));
        for name in ["untracked.txt", "gameskills.toml", "gameskills.lock.json"] {
            let previous = fs::read(f.root.join(name)).ok();
            fs::write(f.root.join(name), "input")?;
            assert!(!ok(&f.evidence(id, "validate")?));
            if let Some(previous) = previous {
                fs::write(f.root.join(name), previous)?;
            } else {
                fs::remove_file(f.root.join(name))?;
            }
        }
        let original = f.config.clone();
        f.command("a", &["echo", "ok"], json!({"timeout_seconds":123}))?;
        assert!(!ok(&f.evidence(id, "validate")?));
        f.config = original;
        f.sync_config()?;
        let output = f
            .prepared("evidence", &["validate", id])?
            .env("GAMESKILLS_TEST_INPUT", "different")
            .output()?;
        assert!(!ok(&serde_json::from_slice::<Value>(&output.stdout)?));
        fs::write(f.root.join("source.txt"), "staged")?;
        f.git(&["add", "source.txt"])?;
        fs::write(f.root.join("source.txt"), "original\n")?;
        assert!(!ok(&f.evidence(id, "validate")?));
        f.git(&["reset", "--quiet", "HEAD", "source.txt"])?;
        assert!(ok(&f.evidence(id, "validate")?));
        f.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "new head",
        ])?;
        assert!(!ok(&f.evidence(id, "validate")?));
        Ok(())
    }
    #[test]
    fn input_change_during_successful_command_marks_run_stale() -> Test {
        let mut f = Fixture::new()?;
        f.command("mutate", &["mutate", "source.txt", "changed"], json!({}))?;
        let result = f.run(&["mutate"])?;
        assert_eq!(status(&result, "mutate"), Some("passed"));
        assert_eq!(result.get("status"), Some(&json!("stale")));
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        Ok(())
    }
    #[test]
    fn base_ref_change_invalidates_evidence_with_unchanged_head() -> Test {
        let mut f = Fixture::new()?;
        let head = f.git(&["rev-parse", "HEAD"])?;
        f.git(&["branch", "review-base", &head])?;
        let advanced = f.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            &head,
            "-m",
            "advanced",
        ])?;
        f.command_raw(
            "check",
            json!({"argv":["git","merge-base","--is-ancestor","refs/heads/review-base","HEAD"]}),
        )?;
        let result = f.run(&["check"])?;
        assert!(ok(&result));
        let prior = fs::read(f.record(f.id(&result)?))?;
        f.git(&["update-ref", "refs/heads/review-base", &advanced])?;
        assert_eq!(f.git(&["rev-parse", "HEAD"])?, head);
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        assert_eq!(
            f.invoke("run", &["check", "--resume", f.id(&result)?])?.0,
            2
        );
        assert!(!ok(&f.run(&["check"])?));
        assert_eq!(fs::read(f.record(f.id(&result)?))?, prior);
        Ok(())
    }
    #[test]
    fn unrelated_refs_and_symbolic_head_are_conservative_inputs() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let a = f.run(&["a"])?;
        f.git(&["branch", "unrelated", "HEAD"])?;
        assert!(!ok(&f.evidence(f.id(&a)?, "validate")?));
        let b = f.run(&["a"])?;
        f.git(&["pack-refs", "--all", "--prune"])?;
        assert!(ok(&f.evidence(f.id(&b)?, "validate")?));
        f.git(&["checkout", "--quiet", "unrelated"])?;
        assert!(!ok(&f.evidence(f.id(&b)?, "validate")?));
        Ok(())
    }
    #[test]
    fn ref_change_during_a_command_marks_run_stale() -> Test {
        let mut f = Fixture::new()?;
        f.command_raw(
            "ref",
            json!({"argv":["git","update-ref","refs/heads/new-ref","HEAD"]}),
        )?;
        let result = f.run(&["ref"])?;
        assert_eq!(status(&result, "ref"), Some("passed"));
        assert_eq!(result.get("status"), Some(&json!("stale")));
        Ok(())
    }
    #[test]
    fn resume_reruns_graph_preserves_failed_record_and_rejects_new_inputs() -> Test {
        let mut f = Fixture::new()?;
        f.command("retry", &["retry", ".gameskills/retry-marker"], json!({}))?;
        let failed = f.run(&["retry"])?;
        let prior = fs::read(f.record(f.id(&failed)?))?;
        let resumed = f.run(&["retry", "--resume", f.id(&failed)?])?;
        assert!(ok(&resumed), "{resumed}");
        assert_ne!(f.id(&resumed)?, f.id(&failed)?);
        assert_eq!(fs::read(f.record(f.id(&failed)?))?, prior);
        fs::write(f.root.join("source.txt"), "changed")?;
        assert_eq!(
            f.invoke("run", &["retry", "--resume", f.id(&failed)?])?.0,
            2
        );
        Ok(())
    }
    #[test]
    fn active_run_cannot_be_resumed() -> Test {
        let mut f = Fixture::new()?;
        let ready = f.root.join(".gameskills/ready");
        f.command(
            "wait",
            &[
                "spawn",
                ready.to_str().ok_or("path")?,
                f.root.join(".gameskills/escaped").to_str().ok_or("path")?,
                "hang",
            ],
            json!({}),
        )?;
        let mut child = f.launch("run", &["wait"])?;
        f.ready(&ready)?;
        let id = fs::read_dir(f.root.join(".gameskills/runs"))?
            .next()
            .ok_or("missing run")??
            .file_name()
            .into_string()
            .map_err(|_| "invalid run")?;
        let (code, denied) = f.invoke("run", &["wait", "--resume", &id])?;
        assert_eq!(code, 2);
        assert!(denied.to_string().contains("active run"), "{denied}");
        child.signal(nix::sys::signal::Signal::SIGTERM)?;
        child.collect()?;
        Ok(())
    }
    #[test]
    fn modified_record_and_modified_log_never_validate() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "authentic"], json!({}))?;
        let result = f.run(&["a"])?;
        let path = f.record(f.id(&result)?);
        let original = fs::read(&path)?;
        let mut envelope: Value = serde_json::from_slice(&original)?;
        *envelope
            .pointer_mut("/record/results/a/argv")
            .ok_or("missing argv")? = json!(["invented"]);
        fs::write(&path, serde_json::to_vec(&envelope)?)?;
        assert!(f
            .evidence(f.id(&result)?, "validate")?
            .to_string()
            .contains("digest mismatch"));
        fs::write(&path, original)?;
        fs::write(
            path.parent().ok_or("no dir")?.join("a.stdout.log"),
            "replaced",
        )?;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        Ok(())
    }
    #[test]
    fn record_relocated_to_another_run_or_repository_is_invalid() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let result = f.run(&["a"])?;
        let path = f.record(f.id(&result)?);
        let original = path.parent().ok_or("no run")?;
        let new = "0123456789abcdef0123456789abcdef";
        fs::rename(original, original.with_file_name(new))?;
        assert!(!ok(&f.evidence(new, "validate")?));
        fs::rename(original.with_file_name(new), original)?;
        let moved = f.root.with_file_name("moved-project");
        fs::rename(&f.root, &moved)?;
        f.root = moved;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        Ok(())
    }
    #[test]
    fn state_symlinks_record_symlinks_hardlinks_and_fifo_are_rejected() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let external = f.scratch.path().join("external");
        fs::create_dir(&external)?;
        let saved = f.scratch.path().join("saved-state");
        fs::rename(f.root.join(".gameskills"), &saved)?;
        symlink(&external, f.root.join(".gameskills"))?;
        assert_eq!(f.invoke("run", &["a"])?.0, 2);
        assert_eq!(fs::read_dir(&external)?.count(), 0);
        fs::remove_file(f.root.join(".gameskills"))?;
        fs::rename(saved, f.root.join(".gameskills"))?;
        let result = f.run(&["a"])?;
        let log = f.record(f.id(&result)?).with_file_name("a.stdout.log");
        let original = fs::read(&log)?;
        let outside = external.join("log");
        fs::write(&outside, original)?;
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600))?;
        fs::remove_file(&log)?;
        symlink(&outside, &log)?;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        fs::remove_file(&log)?;
        fs::hard_link(&outside, &log)?;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        fs::remove_file(&log)?;
        nix::unistd::mkfifo(
            &log,
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )?;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        Ok(())
    }
    #[test]
    fn evidence_path_traversal_is_rejected() -> Test {
        let f = Fixture::new()?;
        for id in [
            "../escape",
            "/absolute",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "ABCDEF",
            "..",
        ] {
            assert_eq!(f.invoke("evidence", &["validate", id])?.0, 2);
        }
        Ok(())
    }
    #[test]
    fn changed_external_executable_invalidates_evidence() -> Test {
        let mut f = Fixture::new()?;
        let executable = f.scratch.path().join("external command");
        fs::copy(&f.probe, &executable)?;
        f.command_raw("external", json!({"argv":[executable,"echo","original"]}))?;
        let result = f.run(&["external"])?;
        assert!(ok(&result), "{result}");
        use std::io::Write;
        fs::OpenOptions::new()
            .append(true)
            .open(&executable)?
            .write_all(b"changed bytes")?;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        Ok(())
    }
    #[test]
    fn relative_path_entry_resolves_from_actual_command_cwd() -> Test {
        let mut f = Fixture::new()?;
        let executable = f.root.join("relative-program");
        fs::copy(&f.probe, &executable)?;
        f.command_raw(
            "relative",
            json!({"argv":["relative-program","echo","local"]}),
        )?;
        let path = format!(".:{}", std::env::var("PATH")?);
        let output = f
            .prepared("run", &["relative"])?
            .env("PATH", path)
            .output()?;
        let result: Value = serde_json::from_slice(&output.stdout)?;
        assert!(ok(&result), "{result}");
        let envelope: Value = serde_json::from_slice(&fs::read(f.record(f.id(&result)?))?)?;
        assert_eq!(
            envelope.pointer("/record/identity/executables/relative/path"),
            Some(&json!(executable))
        );
        Ok(())
    }
    #[test]
    fn tracked_parent_replaced_by_symlink_is_not_followed_during_fingerprinting() -> Test {
        let mut f = Fixture::new()?;
        fs::create_dir(f.root.join("nested"))?;
        fs::write(f.root.join("nested/input.txt"), "tracked")?;
        f.commit("nested")?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let result = f.run(&["a"])?;
        fs::rename(
            f.root.join("nested"),
            f.root.join(".gameskills/moved-source"),
        )?;
        symlink(".gameskills/moved-source", f.root.join("nested"))?;
        assert!(!ok(&f.evidence(f.id(&result)?, "validate")?));
        assert_eq!(f.invoke("run", &["a"])?.0, 2);
        Ok(())
    }
    #[test]
    fn hard_killed_runner_cleans_descendants_and_releases_locks_only_after_cleanup() -> Test {
        let mut f = Fixture::new()?;
        let ready = f.root.join(".gameskills/ready");
        let escaped = f.root.join(".gameskills/escaped");
        f.command(
            "wait",
            &[
                "spawn",
                ready.to_str().ok_or("path")?,
                escaped.to_str().ok_or("path")?,
                "once",
            ],
            json!({"resources":[format!("test:{}",f.root.display())]}),
        )?;
        let mut child = f.launch("run", &["wait"])?;
        f.ready(&ready)?;
        let id = fs::read_dir(f.root.join(".gameskills/runs"))?
            .next()
            .ok_or("missing run")??
            .file_name()
            .into_string()
            .map_err(|_| "invalid id")?;
        child.signal(nix::sys::signal::Signal::SIGKILL)?;
        child.collect()?;
        assert!(!ok(&f.evidence(&id, "validate")?));
        let lock = fs::File::open(f.record(&id).with_file_name("active.lock"))?;
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            if rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)
                .is_ok()
            {
                break;
            }
            if Instant::now() > deadline {
                return Err("supervisor retained active lock after cleanup deadline".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(lock);
        std::thread::sleep(Duration::from_millis(1250));
        assert!(!escaped.exists());
        let before = fs::read(f.record(&id))?;
        let shown = f.evidence(&id, "show")?;
        assert_eq!(shown.pointer("/record/status"), Some(&json!("running")));
        assert_eq!(fs::read(f.record(&id))?, before);
        let resumed = f.run(&["wait", "--resume", &id])?;
        assert!(ok(&resumed), "{resumed}");
        assert_ne!(f.id(&resumed)?, id);
        assert_eq!(fs::read(f.record(&id))?, before);
        Ok(())
    }
    #[test]
    fn resource_initialization_failure_preserves_a_failed_record() -> Test {
        use sha2::{Digest, Sha256};
        let mut f = Fixture::new()?;
        let resource = format!("test:{}", f.root.display());
        f.command(
            "a",
            &["echo", "must not execute"],
            json!({"resources":[resource]}),
        )?;
        let directory = PathBuf::from("/tmp").canonicalize()?.join(format!(
            "gameskills-resources-{}",
            rustix::process::getuid().as_raw()
        ));
        fs::create_dir_all(&directory)?;
        let name = format!(
            "{:x}.lock",
            Sha256::digest(serde_json::to_vec(&json!(["global", resource]))?)
        );
        let path = directory.join(name);
        symlink(f.scratch.path().join("missing"), &path)?;
        let result = f.run(&["a"])?;
        fs::remove_file(path)?;
        assert!(!ok(&result));
        assert_eq!(status(&result, "a"), Some("skipped"));
        assert!(result.get("runner_error").is_some_and(Value::is_string));
        let shown = f.evidence(f.id(&result)?, "show")?;
        assert_eq!(shown.pointer("/record/status"), Some(&json!("failed")));
        Ok(())
    }
    #[test]
    fn missing_executable_is_failed_observation_with_real_error_log() -> Test {
        let mut f = Fixture::new()?;
        f.command_raw(
            "missing",
            json!({"argv":["gameskills-definitely-missing-command"]}),
        )?;
        let result = f.run(&["missing"])?;
        assert_eq!(status(&result, "missing"), Some("failed"), "{result}");
        assert_eq!(
            result.pointer("/results/missing/exit_code"),
            Some(&json!(127))
        );
        assert!(f
            .log(&result, "missing", "stderr")?
            .contains("cannot execute"));
        Ok(())
    }
    #[test]
    fn historical_python_records_are_shown_but_never_reused() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let result = f.run(&["a"])?;
        let path = f.record(f.id(&result)?);
        let mut record: Value = serde_json::from_slice(&fs::read(&path)?)?;
        *record
            .pointer_mut("/record/schema_version")
            .ok_or("schema")? = json!(1);
        record
            .pointer_mut("/record")
            .and_then(Value::as_object_mut)
            .ok_or("record")?
            .remove("runtime");
        fs::write(&path, serde_json::to_vec(&record)?)?;
        let bytes = fs::read(&path)?;
        let shown = f.evidence(f.id(&result)?, "show")?;
        assert!(!ok(&shown));
        assert_eq!(shown.pointer("/record/schema_version"), Some(&json!(1)));
        assert_eq!(f.invoke("run", &["a", "--resume", f.id(&result)?])?.0, 2);
        assert_eq!(fs::read(path)?, bytes);
        Ok(())
    }
    #[test]
    fn supervisor_rejects_an_unbound_invocation() -> Test {
        let f = Fixture::new()?;
        let output = Command::new(&f.probe)
            .arg("__runner-supervisor")
            .stdin(Stdio::null())
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        Ok(())
    }
    #[test]
    fn setup_lock_serializes_run_registration() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let path = f.root.join(".gameskills/setup.lock");
        let guard = fs::OpenOptions::new().read(true).write(true).open(path)?;
        rustix::fs::flock(&guard, rustix::fs::FlockOperation::LockExclusive)?;
        let mut run = f.launch("run", &["a"])?;
        std::thread::sleep(Duration::from_millis(150));
        assert!(!f.root.join(".gameskills/runs").exists());
        drop(guard);
        let (_, result) = run.collect()?;
        assert!(ok(&result), "{result}");
        Ok(())
    }
    #[test]
    fn deleted_tracked_parent_is_fingerprinted_as_missing() -> Test {
        let mut f = Fixture::new()?;
        fs::create_dir(f.root.join("nested"))?;
        fs::write(f.root.join("nested/input"), "tracked")?;
        f.commit("nested")?;
        f.command("a", &["echo", "ok"], json!({}))?;
        let before = f.run(&["a"])?;
        fs::remove_dir_all(f.root.join("nested"))?;
        assert!(!ok(&f.evidence(f.id(&before)?, "validate")?));
        let after = f.run(&["a"])?;
        assert!(ok(&after), "{after}");
        Ok(())
    }
    #[test]
    fn global_resources_span_repositories_and_project_resources_do_not() -> Test {
        let mut a = Fixture::new()?;
        let mut b = Fixture::new()?;
        for (prefix, overlap) in [("test", false), ("project", true)] {
            let resource = format!("{prefix}:{}", a.root.display());
            for fixture in [&mut a, &mut b] {
                fixture.command(
                    "work",
                    &["interval", "300"],
                    json!({"resources":[resource]}),
                )?;
            }
            let (mut run_a, mut run_b) = (a.launch("run", &["work"])?, b.launch("run", &["work"])?);
            let (_, result_a) = run_a.collect()?;
            let (_, result_b) = run_b.collect()?;
            assert!(ok(&result_a) && ok(&result_b), "{result_a} {result_b}");
            let (first, second) = (
                interval(&a, &result_a, "work")?,
                interval(&b, &result_b, "work")?,
            );
            assert_eq!(first.0.max(second.0) < first.1.min(second.1), overlap);
        }
        Ok(())
    }
    #[test]
    fn real_installation_rechecks_config_under_lock_and_rejects_setup_during_run() -> Test {
        let mut f = Fixture::new()?;
        let config_path = f.root.join("gameskills.toml");
        let ready = f.root.join(".gameskills/ready");
        let escaped = f.root.join(".gameskills/escaped");
        f.command("a", &["echo", "ok"], json!({}))?;
        f.command(
            "hold",
            &[
                "spawn",
                ready.to_str().ok_or("path")?,
                escaped.to_str().ok_or("path")?,
                "hang",
            ],
            json!({}),
        )?;
        let text = fs::read_to_string(&config_path)?;
        f.config = gameskills_cli::installation::ready_config(&f.root)?;
        let result = f.run(&["a"])?;
        assert!(ok(&result), "{result}");
        fs::write(&config_path, text.replace("\"ok\"", "\"changed\""))?;
        let (code, error) = f.invoke("run", &["a"])?;
        assert_eq!(code, 2);
        assert!(
            error
                .to_string()
                .contains("changed before run registration"),
            "{error}"
        );
        f.config = gameskills_cli::installation::ready_config(&f.root)?;
        let mut child = f.launch("run", &["hold"])?;
        f.ready(&ready)?;
        let setup = gameskills_cli::installation::execute(&f.root, "setup", &["--apply".into()]);
        assert!(setup.err().is_some_and(|error| error.contains("active")));
        child.signal(nix::sys::signal::Signal::SIGTERM)?;
        child.collect()?;
        Ok(())
    }
    #[test]
    fn deleting_both_managed_files_cannot_bypass_registration_readiness() -> Test {
        let mut f = Fixture::new()?;
        f.command("a", &["echo", "must not run"], json!({}))?;
        let mut prepared = f.prepared("run", &["a"])?;
        fs::remove_file(f.root.join("gameskills.toml"))?;
        fs::remove_file(f.root.join("gameskills.lock.json"))?;
        let mut child = ChildGuard(Some(prepared.spawn()?));
        let (code, error) = child.collect()?;
        assert_eq!(code, 2, "{error}");
        assert!(!f.root.join(".gameskills/runs").exists());
        Ok(())
    }
}

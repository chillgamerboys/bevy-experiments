//! Exercise an external prebuilt consumer with Cargo and interpreters absent from PATH.

#[cfg(unix)]
mod posix {
    use serde_json::{json, Value};
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        process::{Command, Output},
    };

    type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

    struct Consumer {
        _temp: tempfile::TempDir,
        root: PathBuf,
        binary: PathBuf,
        bin: PathBuf,
        home: PathBuf,
    }

    impl Consumer {
        fn new() -> Result<Self> {
            let temp = tempfile::Builder::new()
                .prefix("gameskills adopter ")
                .tempdir()?;
            let root = temp.path().join("game");
            let bin = temp.path().join("runtime-path");
            let home = temp.path().join("user-home");
            for path in [&root, &bin, &home] {
                fs::create_dir(path)?;
            }
            let source = std::env::var_os("GAMESKILLS_CANDIDATE_BINARY")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_gameskills")));
            let binary = bin.join("gameskills");
            fs::copy(source, &binary)?;
            let git = std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH unavailable")?)
                .map(|directory| directory.join("git"))
                .find(|path| path.is_file())
                .ok_or("Git unavailable")?
                .canonicalize()?;
            std::os::unix::fs::symlink(git, bin.join("git"))?;
            let value = Self {
                _temp: temp,
                root,
                binary,
                bin,
                home,
            };
            value.git(&value.root, &["init", "-q"])?;
            value.git(&value.root, &["config", "user.name", "Rust adoption"])?;
            value.git(
                &value.root,
                &["config", "user.email", "adoption@example.invalid"],
            )?;
            value.git(&value.root, &["config", "commit.gpgsign", "false"])?;
            fs::write(value.root.join(".gitignore"), ".gameskills/\n")?;
            fs::write(
                value.root.join("README.md"),
                "A fresh Bevy project fixture.\n",
            )?;
            value.commit("Initial consumer")?;
            for program in ["cargo", "rustc", "python", "python3"] {
                assert!(!value.bin.join(program).exists());
            }
            Ok(value)
        }

        fn command(&self, program: &Path, cwd: &Path) -> Command {
            let mut command = Command::new(program);
            command
                .current_dir(cwd)
                .env_clear()
                .env("PATH", &self.bin)
                .env("HOME", &self.home)
                .env("LANG", "C")
                .env("GIT_CONFIG_NOSYSTEM", "1");
            command
        }

        fn git(&self, cwd: &Path, args: &[&str]) -> Result<String> {
            let output = self
                .command(&self.bin.join("git"), cwd)
                .args(args)
                .output()?;
            Self::success(&output)?;
            Ok(String::from_utf8(output.stdout)?.trim().to_owned())
        }

        fn success(output: &Output) -> Result {
            if !output.status.success() {
                return Err(format!(
                    "status {}: {} {}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            Ok(())
        }

        fn cli(&self, args: &[&str]) -> Result<Value> {
            let output = self.command(&self.binary, &self.root).args(args).output()?;
            Self::success(&output)?;
            let value: Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(value.get("ok"), Some(&json!(true)), "{value}");
            Ok(value)
        }

        fn commit(&self, message: &str) -> Result<String> {
            self.git(&self.root, &["add", "."])?;
            self.git(&self.root, &["commit", "-qm", message])?;
            self.git(&self.root, &["rev-parse", "HEAD"])
        }
    }

    #[test]
    fn prebuilt_baseline_selection_rollback_and_evidence_without_build_tools() -> Result {
        let consumer = Consumer::new()?;
        let output = consumer
            .command(&consumer.binary, &consumer.home)
            .arg("--version")
            .output()?;
        Consumer::success(&output)?;
        consumer.cli(&["catalog"])?;
        consumer.cli(&["setup"])?;
        assert!(!consumer.root.join("gameskills.lock.json").exists());
        consumer.cli(&["setup", "--apply"])?;
        consumer.cli(&["status"])?;
        let initial_config = fs::read_to_string(consumer.root.join("gameskills.toml"))?;
        let initial_lock: Value =
            serde_json::from_slice(&fs::read(consumer.root.join("gameskills.lock.json"))?)?;
        assert_eq!(initial_lock.get("schema_version"), Some(&json!(2)));
        let owned = consumer.root.join("AGENTS.md");
        fs::write(&owned, "Keep game-owned guidance.\n")?;
        consumer.cli(&[
            "setup",
            "--packages",
            "gameskills",
            "gameskills-ui",
            "--apply",
        ])?;
        consumer.cli(&["status"])?;
        let export = consumer.home.join("pinned instructions");
        consumer.cli(&["bundle", "--out", export.to_str().ok_or("export UTF-8")?])?;
        consumer.cli(&[
            "setup",
            "--packages",
            "gameskills",
            "--bundle",
            export.to_str().ok_or("export UTF-8")?,
            "--apply",
        ])?;
        // Reproduce an interrupted transaction after its config write but before
        // its lock write; recovery must restore both original files.
        let lock_before = fs::read_to_string(consumer.root.join("gameskills.lock.json"))?;
        let journal = json!({
            "gameskills.toml": {"before": initial_config, "after": "schema_version=1\n"},
            "gameskills.lock.json": {"before": lock_before, "after": null}
        });
        fs::write(
            consumer.root.join(".gameskills/setup-transaction.json"),
            serde_json::to_vec(&journal)?,
        )?;
        fs::write(consumer.root.join("gameskills.toml"), "schema_version=1\n")?;
        let recovering = consumer.cli(&["setup", "--recover"])?;
        assert_eq!(recovering.get("recovered"), Some(&json!(true)));
        assert_eq!(
            fs::read_to_string(consumer.root.join("gameskills.lock.json"))?,
            lock_before
        );
        consumer.cli(&["status"])?;
        assert_eq!(fs::read_to_string(&owned)?, "Keep game-owned guidance.\n");
        assert_eq!(
            fs::read_to_string(consumer.root.join("gameskills.toml"))?,
            initial_config
        );
        let config = format!("{initial_config}\n[commands.version]\nargv = [{}, \"--version\"]\ntimeout_seconds = 10\n", serde_json::to_string(&consumer.binary)?);
        fs::write(consumer.root.join("gameskills.toml"), config)?;
        consumer.commit("Adopt the standalone Rust candidate")?;
        consumer.cli(&["config"])?;
        let run = consumer.cli(&["run", "version", "--max-workers", "1"])?;
        let id = run.get("run_id").and_then(Value::as_str).ok_or("run ID")?;
        assert_eq!(
            run.pointer("/results/version/status"),
            Some(&json!("passed"))
        );
        consumer.cli(&["evidence", "validate", id])?;
        let record: Value = serde_json::from_slice(&fs::read(
            consumer
                .root
                .join(".gameskills/runs")
                .join(id)
                .join("record.json"),
        )?)?;
        assert_eq!(record.pointer("/record/schema_version"), Some(&json!(2)));
        assert_eq!(record.pointer("/record/runtime"), Some(&json!("rust")));
        fs::write(
            consumer.root.join("README.md"),
            "A later game edit invalidates prior evidence.\n",
        )?;
        let stale = consumer
            .command(&consumer.binary, &consumer.root)
            .args(["evidence", "validate", id])
            .output()?;
        assert!(!stale.status.success());
        let stale: Value = serde_json::from_slice(&stale.stdout)?;
        assert_eq!(stale.get("ok"), Some(&json!(false)));
        Ok(())
    }
}

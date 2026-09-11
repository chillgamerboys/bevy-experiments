//! Real Git worktrees exercise queue observations, failure atomicity and history.
use gameskills_cli::workflow::{execute, validate_plan};
use serde_json::{json, Value};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn at<'a>(value: &'a Value, pointer: &str) -> &'a Value {
    value.pointer(pointer).unwrap_or(&Value::Null)
}
#[cfg(unix)]
fn set(value: &mut Value, key: &str, replacement: Value) {
    value
        .as_object_mut()
        .expect("test object")
        .insert(key.into(), replacement);
}
fn changed(mut value: Value, changes: Value) -> Value {
    value
        .as_object_mut()
        .expect("test object")
        .extend(changes.as_object().expect("test changes").clone());
    value
}
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if std::env::consts::OS.eq("windows") {
                "NUL"
            } else {
                "/dev/null"
            },
        )
        .output()
        .expect("git is available");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git UTF-8")
        .trim()
        .to_owned()
}
fn config_text(value: &Value) -> String {
    format!("schema_version = 1\npackages = {}\n[creative]\ndefault_level = {}\n[dispatch]\nenabled = {}\nmax_workers = {}\n", at(value,"/packages"), at(value,"/creative/default_level"), at(value,"/dispatch/enabled"), at(value,"/dispatch/max_workers"))
}
struct Fixture {
    _temp: TempDir,
    home: PathBuf,
    root: PathBuf,
    base: String,
    config: Value,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let home = temp.path().canonicalize().expect("canonical temp");
        let root = home.join("repo");
        fs::create_dir(&root).expect("repo");
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.name", "Test"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        fs::write(root.join("README.md"), "initial\n").expect("source");
        let config = json!({"dispatch":{"enabled":true,"max_workers":5},"packages":["gameskills","gameskills-ui"],"creative":{"default_level":2}});
        let config_source = config_text(&config);
        fs::write(root.join("gameskills.toml"), &config_source).expect("real config");
        let config = serde_json::to_value(
            gameskills_cli::config::parse(&config_source).expect("parse config"),
        )
        .expect("JSON config");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "initial"]);
        let base = git(&root, &["rev-parse", "HEAD"]);
        Self {
            _temp: temp,
            home,
            root,
            base,
            config,
        }
    }
    fn order(&self, name: &str) -> Value {
        json!({"id":name,"goal":"Bounded change","artifact":"Committed source","target":"game","creative_scope":"Preserve rules","owner":{"kind":"agent","name":name},"execution":{"role":"implementation","effort":"default"},"decisions":[],"investigation":["Observed current source"],"references":[],"acceptance":["Check changed behavior"],"expected_evidence":["Source-bound check"],"coordination":["Coordinate shared changes"],"resources":[],"files":[{"path":format!("{name}.rs")}],"dispatch_blockers":[],"merge_blockers":[]})
    }
    fn plan(&self, orders: Vec<Value>) -> Value {
        json!({"schema_version":1,"id":"wave","goal":"Improve game","delivery_target":"pr","packages":["gameskills"],"decisions":[],"versions":{"bevy":"0.19","gamekit":null},"repository":{"root":self.root,"base_commit":self.base,"source_commit":self.base},"orders":orders})
    }
    fn input(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.home.join(name);
        fs::write(&path, serde_json::to_vec_pretty(value).expect("JSON")).expect("input");
        path
    }
    #[cfg(unix)]
    fn create(&self, orders: Vec<Value>) -> Value {
        self.create_plan(&self.plan(orders))
    }
    #[cfg(unix)]
    fn create_plan(&self, plan: &Value) -> Value {
        let file = self.input("plan.json", plan);
        execute(
            &self.root,
            &self.config,
            "queue",
            &["create".into(), "--file".into(), file.into_os_string()],
        )
        .expect("create queue")
    }
    fn tree(&self, name: &str) -> PathBuf {
        let path = self.home.join(name);
        git(
            &self.root,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                name,
                path.to_str().expect("UTF-8"),
                &self.base,
            ],
        );
        path
    }
    #[cfg(unix)]
    fn path(&self) -> PathBuf {
        self.root.join(".gameskills/queues/wave.json")
    }
    #[cfg(unix)]
    fn read(&self) -> Value {
        serde_json::from_slice(&fs::read(self.path()).expect("queue bytes")).expect("queue JSON")
    }
    #[cfg(unix)]
    fn revision(&self) -> u64 {
        at(&self.read(), "/revision").as_u64().expect("revision")
    }
    #[cfg(unix)]
    fn status(&self, config: &Value) -> Result<Value, String> {
        execute(
            &self.root,
            config,
            "queue",
            &["status".into(), "wave".into()],
        )
    }
    #[cfg(unix)]
    fn mutate_config(
        &self,
        action: &str,
        order: &str,
        payload: Value,
        tree: Option<&Path>,
        revision: u64,
        config: &Value,
    ) -> Result<Value, String> {
        let mut args = vec![OsString::from(action), "wave".into()];
        if action != "inject" {
            args.push(order.into());
        }
        args.extend(["--expected-revision".into(), revision.to_string().into()]);
        if let Some(tree) = tree {
            args.extend(["--worktree".into(), tree.as_os_str().to_owned()]);
        } else {
            let file = self.input(&format!("{action}-{order}-{revision}.json"), &payload);
            args.extend(["--file".into(), file.into_os_string()]);
        }
        let source = config_text(config);
        if fs::read_to_string(self.root.join("gameskills.toml")).expect("config") == source {
        } else {
            fs::write(self.root.join("gameskills.toml"), source).expect("update fixture config");
        }
        execute(&self.root, config, "queue", &args)
    }
    #[cfg(unix)]
    fn mutate(
        &self,
        action: &str,
        order: &str,
        payload: Value,
        tree: Option<&Path>,
    ) -> Result<Value, String> {
        self.mutate_config(action, order, payload, tree, self.revision(), &self.config)
    }
    #[cfg(unix)]
    fn start(&self, order: &str, tree: &Path) -> Value {
        self.mutate("start", order, Value::Null, Some(tree))
            .expect("start")
    }
    #[cfg(unix)]
    fn report_payload(&self, tree: &Path) -> Value {
        json!({"summary":"Implemented bounded work","worktree":tree,"head":git(tree,&["rev-parse","HEAD"]),"base_commit":self.base,"checks_running":false,"evidence":[{"kind":"test","reference":"run:caller-reference","summary":"Caller reports check success"}]})
    }
    #[cfg(unix)]
    fn report(&self, order: &str, tree: &Path) -> Value {
        self.mutate("report", order, self.report_payload(tree), None)
            .expect("report")
    }
    #[cfg(unix)]
    fn block(&self, order: &str, reason: &str) -> Value {
        self.mutate(
            "block",
            order,
            json!({"reason":reason,"checks_running":false}),
            None,
        )
        .expect("block")
    }
    fn commit(&self, tree: &Path, file: &str, content: &str) -> String {
        fs::write(tree.join(file), content).expect("source edit");
        git(tree, &["add", file]);
        git(tree, &["commit", "-qm", &format!("change {file}")]);
        git(tree, &["rev-parse", "HEAD"])
    }
    #[cfg(unix)]
    fn integrate(&self, order: &str, head: &str) -> Value {
        git(&self.root, &["merge", "--no-edit", head]);
        self.mutate("integrated",order,json!({"summary":"Coordinator observed integration","head":git(&self.root,&["rev-parse","HEAD"]),"source_head":head,"evidence_reference":"review-and-combined-checks-reference"}),None).expect("integration")
    }
    #[cfg(unix)]
    fn unchanged(&self, contains: &str, callback: impl FnOnce() -> Result<Value, String>) {
        let before = fs::read(self.path()).expect("original bytes");
        let error = callback().expect_err("expected rejection");
        assert!(
            error.contains(contains),
            "expected {contains:?}, got {error:?}"
        );
        assert_eq!(fs::read(self.path()).expect("unchanged bytes"), before);
    }
}

#[test]
fn plan_preserves_input_and_normalizes_identity() {
    let f = Fixture::new();
    let plan = f.plan(vec![f.order("one")]);
    let original = plan.clone();
    let validated = validate_plan(&plan, &f.root, &f.config).expect("valid plan");
    assert_eq!(plan, original);
    assert_eq!(at(&validated, "/creative_level"), 2);
    assert_eq!(at(&validated, "/orders/0/creative_level"), 2);
    assert_eq!(
        at(&validated, "/repository/common_dir"),
        f.root.join(".git").to_str().expect("path")
    );
    assert!(!f.root.join(".gameskills").exists());
}
#[test]
fn invalid_ids_dependencies_cycles_and_scalar_types() {
    let f = Fixture::new();
    for id in ["../escape", "a/b", ".", "x\\y", "", &"a".repeat(65)] {
        let plan = changed(f.plan(vec![f.order("a")]), json!({"id":id}));
        assert!(validate_plan(&plan, &f.root, &f.config).is_err());
    }
    for orders in [
        vec![changed(
            f.order("a"),
            json!({"dispatch_blockers":["missing"]}),
        )],
        vec![
            changed(f.order("a"), json!({"dispatch_blockers":["b"]})),
            changed(f.order("b"), json!({"merge_blockers":["a"]})),
        ],
        vec![f.order("a"), f.order("a")],
    ] {
        assert!(validate_plan(&f.plan(orders), &f.root, &f.config).is_err());
    }
    for (key, value) in [
        ("schema_version", json!(true)),
        ("creative_level", json!(2.0)),
        ("creative_level", json!(5)),
        ("packages", json!(["gameskills", "gameskills"])),
        ("goal", json!("bad\ntext")),
        ("versions", json!({"bevy":false})),
    ] {
        assert!(
            validate_plan(
                &changed(f.plan(vec![f.order("a")]), json!({key:value})),
                &f.root,
                &f.config
            )
            .is_err(),
            "{key}"
        );
    }
    for (key, value) in [
        ("dispatch_blockers", json!(["a"])),
        ("dispatch_blockers", json!(["b", "b"])),
        ("files", json!([{ "path":"a.rs","lines":[true,3]}])),
        ("acceptance", json!([])),
        ("owner", json!({"kind":"robot","name":"test"})),
    ] {
        assert!(
            validate_plan(
                &f.plan(vec![
                    changed(f.order("a"), json!({key:value})),
                    f.order("b")
                ]),
                &f.root,
                &f.config
            )
            .is_err(),
            "{key}"
        );
    }
    assert!(!f.root.join(".gameskills").exists());
}
#[test]
fn line_ranges_directories_and_literal_ownership() {
    let f = Fixture::new();
    let first = changed(
        f.order("a"),
        json!({"files":[{"path":"src/game.rs","lines":[1,20]}]}),
    );
    let separate = changed(
        f.order("b"),
        json!({"files":[{"path":"src/game.rs","lines":[21,50]}]}),
    );
    validate_plan(&f.plan(vec![first.clone(), separate]), &f.root, &f.config)
        .expect("separate lines");
    for files in [
        json!([{"path":"src/game.rs","lines":[20,30]}]),
        json!([{"path":"src/game.rs"}]),
        json!([{"path":"src/"}]),
    ] {
        assert!(validate_plan(
            &f.plan(vec![
                first.clone(),
                changed(f.order("b"), json!({"files":files}))
            ]),
            &f.root,
            &f.config
        )
        .is_err());
    }
    validate_plan(
        &f.plan(vec![
            first.clone(),
            changed(
                f.order("b"),
                json!({"files":at(&first,"/files"),"dispatch_blockers":["a"]}),
            ),
        ]),
        &f.root,
        &f.config,
    )
    .expect("sequenced lines");
    for path in [
        "../src/x",
        "/src/x",
        "a/../x",
        "a//x",
        "*.rs",
        ".git/config",
        ".gameskills",
        "a//",
        "C:/escape",
        "foo\\bar",
    ] {
        assert!(
            validate_plan(
                &f.plan(vec![changed(
                    f.order("a"),
                    json!({"files":[{"path":path}]})
                )]),
                &f.root,
                &f.config
            )
            .is_err(),
            "{path}"
        );
    }
}
#[test]
fn repository_source_base_and_commit_identity() {
    let f = Fixture::new();
    let tree = f.tree("other");
    let plan = changed(
        f.plan(vec![f.order("a")]),
        json!({"repository":{"root":tree,"base_commit":f.base,"source_commit":f.base}}),
    );
    assert!(validate_plan(&plan, &f.root, &f.config).is_err());
    let future = f.commit(&tree, "other.rs", "future");
    for (base, source, expected) in [(&f.base, &future, "stale"), (&future, &f.base, "descend")] {
        let plan = changed(
            f.plan(vec![f.order("a")]),
            json!({"repository":{"root":f.root,"base_commit":base,"source_commit":source}}),
        );
        assert!(validate_plan(&plan, &f.root, &f.config)
            .expect_err("bad identity")
            .contains(expected));
    }
    for bad in [
        "HEAD".to_owned(),
        "a".repeat(40),
        f.base.to_ascii_uppercase(),
    ] {
        let plan = changed(
            f.plan(vec![f.order("a")]),
            json!({"repository":{"root":f.root,"base_commit":bad,"source_commit":f.base}}),
        );
        assert!(validate_plan(&plan, &f.root, &f.config).is_err());
    }
}

#[cfg(unix)]
mod posix {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    #[test]
    fn ownership_rejects_symlink_parent_and_leaf() {
        let f = Fixture::new();
        symlink(f.root.join("README.md"), f.root.join("alias")).expect("alias");
        symlink(&f.home, f.root.join("outside")).expect("directory alias");
        for path in ["alias", "outside/new.rs"] {
            assert!(validate_plan(
                &f.plan(vec![changed(
                    f.order("a"),
                    json!({"files":[{"path":path}]})
                )]),
                &f.root,
                &f.config
            )
            .expect_err("symbolic path")
            .contains("symbolic"));
        }
    }
    #[test]
    fn injection_atomic_add_only_and_source_bound() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let addition = changed(
            f.order("two"),
            json!({"reason":"Verified additional scope","verified_source":f.base}),
        );
        f.mutate("inject", "", addition.clone(), None)
            .expect("inject");
        assert_eq!(at(&f.read(), "/revision"), 2);
        assert_eq!(
            at(&f.read(), "/orders").as_object().expect("orders").len(),
            2
        );
        f.unchanged("add-only", || f.mutate("inject", "", addition, None));
        let bad = changed(
            f.order("three"),
            json!({"files":[{"path":"one.rs"}],"reason":"Need another fix","verified_source":f.base}),
        );
        f.unchanged("collision", || f.mutate("inject", "", bad.clone(), None));
        f.mutate(
            "inject",
            "",
            changed(bad, json!({"dispatch_blockers":["one"]})),
            None,
        )
        .expect("sequenced injection");
        let stale = changed(
            f.order("four"),
            json!({"reason":"Addition","verified_source":"0".repeat(40)}),
        );
        f.unchanged("verified_source", || {
            f.mutate("inject", "", stale.clone(), None)
        });
        f.unchanged("stale revision", || {
            f.mutate_config("inject", "", stale, None, 1, &f.config)
        });
        assert_eq!(
            at(&f.read(), "/orders/one/spec/files"),
            &json!([{"path":"one.rs"}])
        );
    }
    #[test]
    fn injected_resources_require_sequencing_and_integrated_territory_is_reusable() {
        let f = Fixture::new();
        f.create(vec![changed(
            f.order("one"),
            json!({"resources":["gpu/window"]}),
        )]);
        let addition = changed(
            f.order("two"),
            json!({"resources":["gpu/window"],"reason":"Visual check","verified_source":f.base}),
        );
        f.unchanged("resource collision", || {
            f.mutate("inject", "", addition.clone(), None)
        });
        f.mutate(
            "inject",
            "",
            changed(addition, json!({"dispatch_blockers":["one"]})),
            None,
        )
        .expect("sequenced resource");
        let tree = f.tree("one");
        f.start("one", &tree);
        f.report("one", &tree);
        f.integrate("one", &f.base);
        f.mutate(
            "inject",
            "",
            changed(
                f.order("three"),
                json!({"files":[{"path":"one.rs"}],"reason":"Correction","verified_source":f.base}),
            ),
            None,
        )
        .expect("integrated territory");
        assert_eq!(at(&f.read(), "/orders/three/state"), "pending");
    }
    #[test]
    fn returned_dependency_change_is_rejected() {
        let f = Fixture::new();
        f.create(vec![
            f.order("one"),
            changed(f.order("two"), json!({"dispatch_blockers":["one"]})),
        ]);
        let tree = f.tree("one");
        let consumer = f.tree("two");
        f.start("one", &tree);
        f.report("one", &tree);
        f.commit(&tree, "one.rs", "changed after report");
        f.unchanged("changed since its report", || {
            f.mutate("start", "two", Value::Null, Some(&consumer))
        });
    }
    #[test]
    fn five_worker_cap_and_human_ownership() {
        let f = Fixture::new();
        let mut orders = (0..6)
            .map(|i| f.order(&format!("worker{i}")))
            .collect::<Vec<_>>();
        orders.push(changed(
            f.order("human"),
            json!({"owner":{"kind":"human","name":"Contributor"},"resources":["gpu"]}),
        ));
        orders.push(changed(f.order("visual"), json!({"resources":["gpu"]})));
        f.create(orders);
        f.start("human", &f.root);
        for i in 0..5 {
            let name = format!("worker{i}");
            f.start(&name, &f.tree(&name));
        }
        let view = f.status(&f.config).expect("status");
        assert_eq!(at(&view, "/queue/active_workers"), 5);
        f.unchanged("worker limit", || {
            f.mutate("start", "worker5", Value::Null, Some(&f.tree("worker5")))
        });
        assert!(at(&view, "/queue/orders/visual/waiting_reasons")
            .as_array()
            .expect("reasons")
            .contains(&json!("shared resource held by human")));
        assert!(f
            .status(&changed(
                f.config.clone(),
                json!({"dispatch":{"enabled":true,"max_workers":6}})
            ))
            .is_err());
    }
    #[test]
    fn pending_human_reservation_and_shared_agent_resources() {
        let f = Fixture::new();
        f.create(vec![
            changed(
                f.order("human"),
                json!({"owner":{"kind":"human","name":"Contributor"},"resources":["audio"]}),
            ),
            changed(f.order("one"), json!({"resources":["audio"]})),
            changed(f.order("gpu1"), json!({"resources":["gpu"]})),
            changed(f.order("gpu2"), json!({"resources":["gpu"]})),
        ]);
        f.unchanged("held by human", || {
            f.mutate("start", "one", Value::Null, Some(&f.tree("one")))
        });
        f.start("gpu1", &f.tree("gpu1"));
        let two = f.tree("gpu2");
        f.unchanged("shared resource", || {
            f.mutate("start", "gpu2", Value::Null, Some(&two))
        });
        f.block("gpu1", "Window closed");
        f.start("gpu2", &two);
    }
    #[test]
    fn start_resume_recheck_package_selection_and_dispatch() {
        let f = Fixture::new();
        let plan = changed(
            f.plan(vec![changed(
                f.order("one"),
                json!({"packages":["gameskills-ui"]}),
            )]),
            json!({"packages":["gameskills","gameskills-ui"]}),
        );
        f.create_plan(&plan);
        let tree = f.tree("one");
        let config = changed(f.config.clone(), json!({"packages":["gameskills"]}));
        f.unchanged("not selected", || {
            f.mutate_config(
                "start",
                "one",
                Value::Null,
                Some(&tree),
                f.revision(),
                &config,
            )
        });
        assert_eq!(
            at(
                &f.status(&config).expect("changed status"),
                "/queue/active_workers"
            ),
            0
        );
        let disabled = changed(
            f.config.clone(),
            json!({"dispatch":{"enabled":false,"max_workers":5}}),
        );
        f.unchanged("dispatch is disabled", || {
            f.mutate_config(
                "start",
                "one",
                Value::Null,
                Some(&tree),
                f.revision(),
                &disabled,
            )
        });
        f.start("one", &tree);
        f.block("one", "Pause");
        f.unchanged("not selected", || {
            f.mutate_config(
                "resume",
                "one",
                Value::Null,
                Some(&tree),
                f.revision(),
                &config,
            )
        });
        assert_eq!(
            at(&f.read(), "/orders/one/attempts")
                .as_array()
                .expect("attempts")
                .len(),
            1
        );
        f.mutate("resume", "one", Value::Null, Some(&tree))
            .expect("resume");
        assert_eq!(at(&f.read(), "/orders/one/state"), "running");
    }
    #[test]
    fn returned_human_predecessor_allows_only_sequenced_consumers() {
        let f = Fixture::new();
        let human = changed(
            f.order("human"),
            json!({"owner":{"kind":"human","name":"Contributor"},"files":[{"path":"shared.rs"}],"resources":["gpu"]}),
        );
        let consumer = changed(
            f.order("after"),
            json!({"files":at(&human,"/files"),"resources":["gpu"],"dispatch_blockers":["human"]}),
        );
        f.create(vec![
            human,
            consumer,
            changed(f.order("unrelated"), json!({"resources":["gpu"]})),
        ]);
        let ht = f.tree("human");
        let ct = f.tree("after");
        let ut = f.tree("unrelated");
        f.unchanged("held by human", || {
            f.mutate("start", "after", Value::Null, Some(&ct))
        });
        f.start("human", &ht);
        f.unchanged("held by human", || {
            f.mutate("start", "after", Value::Null, Some(&ct))
        });
        let head = f.commit(&ht, "shared.rs", "human result");
        f.report("human", &ht);
        f.unchanged("held by human", || {
            f.mutate("start", "unrelated", Value::Null, Some(&ut))
        });
        f.unchanged("does not contain", || {
            f.mutate("start", "after", Value::Null, Some(&ct))
        });
        git(&ct, &["merge", "--ff-only", &head]);
        f.block("human", "Human reviewing follow-up");
        f.unchanged("held by human", || {
            f.mutate("start", "after", Value::Null, Some(&ct))
        });
        f.mutate("resume", "human", Value::Null, Some(&ht))
            .expect("human resume");
        f.report("human", &ht);
        fs::write(ht.join("shared.rs"), "unreported human edit").expect("dirty");
        f.unchanged("changed since its report", || {
            f.mutate("start", "after", Value::Null, Some(&ct))
        });
        git(&ht, &["restore", "shared.rs"]);
        f.start("after", &ct);
        assert_eq!(at(&f.read(), "/orders/human/state"), "reported");
        assert_eq!(at(&f.read(), "/orders/after/state"), "running");
    }
    #[test]
    fn checkout_isolated_same_repository_unique_and_descendant() {
        let f = Fixture::new();
        f.create(vec![f.order("one"), f.order("two")]);
        f.unchanged("isolated", || {
            f.mutate("start", "one", Value::Null, Some(&f.root))
        });
        let clone = f.home.join("clone");
        git(
            &f.root,
            &[
                "clone",
                "-q",
                f.root.to_str().expect("root"),
                clone.to_str().expect("clone"),
            ],
        );
        f.unchanged("different repository", || {
            f.mutate("start", "one", Value::Null, Some(&clone))
        });
        let tree = f.tree("one");
        f.start("one", &tree);
        f.unchanged("already owned", || {
            f.mutate("start", "two", Value::Null, Some(&tree))
        });
        f.block("one", "Paused partial work");
        f.unchanged("already owned", || {
            f.mutate("start", "two", Value::Null, Some(&tree))
        });
    }
    #[test]
    fn full_lifecycle_and_distinct_dependency_gates() {
        let f = Fixture::new();
        f.create(vec![
            f.order("one"),
            changed(f.order("merge-only"), json!({"merge_blockers":["one"]})),
            changed(f.order("after"), json!({"dispatch_blockers":["one"]})),
        ]);
        let mt = f.tree("merge-only");
        f.start("merge-only", &mt);
        f.report("merge-only", &mt);
        f.unchanged("merge dependency", || {
            f.mutate("integrated", "merge-only", json!({}), None)
        });
        let after = f.tree("after");
        f.unchanged("dispatch dependency", || {
            f.mutate("start", "after", Value::Null, Some(&after))
        });
        let tree = f.tree("one");
        f.start("one", &tree);
        let head = f.commit(&tree, "one.rs", "implemented");
        f.report("one", &tree);
        f.unchanged("not an ancestor",||f.mutate("integrated","one",json!({"summary":"claim","head":f.base,"source_head":head,"evidence_reference":"checks"}),None));
        f.unchanged("does not contain", || {
            f.mutate("start", "after", Value::Null, Some(&after))
        });
        git(&after, &["merge", "--ff-only", &head]);
        f.start("after", &after);
        assert_eq!(git(&f.root, &["rev-parse", "HEAD"]), f.base);
        f.integrate("one", &head);
        f.integrate("merge-only", &f.base);
        assert_eq!(at(&f.read(), "/orders/one/state"), "integrated");
        assert_eq!(at(&f.read(), "/orders/after/state"), "running");
    }
    #[test]
    fn resume_identity_and_immutable_report_block_history() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let tree = f.tree("one");
        let other = f.tree("other");
        f.start("one", &tree);
        f.report("one", &tree);
        let first = at(&f.read(), "/orders/one/reports/0").clone();
        f.block("one", "Review fix needed");
        f.unchanged("original worktree", || {
            f.mutate("resume", "one", Value::Null, Some(&other))
        });
        f.mutate("resume", "one", Value::Null, Some(&tree))
            .expect("resume");
        let head = f.commit(&tree, "one.rs", "changed");
        f.report("one", &tree);
        let value = f.read();
        assert_eq!(at(&value, "/orders/one/reports/0"), &first);
        assert_eq!(
            at(&value, "/orders/one/reports/1/identity/head"),
            &json!(head)
        );
        assert!(at(&value, "/orders/one/reports/1/evidence_status")
            .as_str()
            .expect("status")
            .contains("not verified"));
        f.block("one", "Another review finding");
        assert_eq!(
            at(&f.read(), "/orders/one/blocks")
                .as_array()
                .expect("blocks")
                .len(),
            2
        );
        assert_eq!(
            at(&f.read(), "/orders/one/blocks/0/observation/reason"),
            "Review fix needed"
        );
    }
    #[test]
    fn invalid_transitions_incomplete_dirty_and_stale_reports() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let tree = f.tree("one");
        f.unchanged("running state", || {
            f.mutate("report", "one", f.report_payload(&tree), None)
        });
        f.unchanged("blocked state", || {
            f.mutate("resume", "one", Value::Null, Some(&tree))
        });
        f.start("one", &tree);
        f.unchanged("checks_running", || {
            f.mutate(
                "block",
                "one",
                json!({"reason":"Still testing","checks_running":true}),
                None,
            )
        });
        for extra in [
            json!({"head":"0".repeat(40)}),
            json!({"checks_running":true}),
            json!({"evidence":[]}),
            json!({"base_commit":"0".repeat(40)}),
        ] {
            f.unchanged("observation", || {
                f.mutate(
                    "report",
                    "one",
                    changed(f.report_payload(&tree), extra),
                    None,
                )
            });
        }
        fs::write(tree.join("README.md"), "uncommitted").expect("dirty");
        f.unchanged("clean committed", || {
            f.mutate("report", "one", f.report_payload(&tree), None)
        });
        git(&tree, &["restore", "README.md"]);
        fs::write(tree.join("untracked"), "x").expect("untracked");
        f.unchanged("clean committed", || {
            f.mutate("report", "one", f.report_payload(&tree), None)
        });
        fs::remove_file(tree.join("untracked")).expect("remove fixture");
        f.report("one", &tree);
        let config = changed(f.config.clone(), json!({"creative":{"default_level":3}}));
        let payload = json!({"summary":"Integrated","head":f.base,"source_head":f.base,"evidence_reference":"actual checks"});
        f.unchanged("configuration changed", || {
            f.mutate_config(
                "integrated",
                "one",
                payload.clone(),
                None,
                f.revision(),
                &config,
            )
        });
        f.commit(&tree, "one.rs", "post-report");
        f.unchanged("source or configuration changed", || {
            f.mutate("integrated", "one", payload, None)
        });
    }
    #[test]
    fn parallel_mutations_compare_and_swap_exactly_one_revision() {
        let f = Fixture::new();
        f.create(vec![f.order("one"), f.order("two")]);
        let one = f.tree("one");
        let two = f.tree("two");
        let outcomes = std::thread::scope(|scope| {
            let a = scope
                .spawn(|| f.mutate_config("start", "one", Value::Null, Some(&one), 1, &f.config));
            let b = scope
                .spawn(|| f.mutate_config("start", "two", Value::Null, Some(&two), 1, &f.config));
            vec![a.join().expect("thread"), b.join().expect("thread")]
        });
        assert_eq!(outcomes.iter().filter(|o| o.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| o.as_ref().is_err_and(|e| e.contains("stale revision")))
                .count(),
            1
        );
        let queue = f.read();
        assert_eq!(at(&queue, "/revision"), 2);
        assert_eq!(at(&queue, "/events").as_array().expect("events").len(), 2);
        assert_eq!(
            at(
                &f.status(&f.config).expect("status"),
                "/queue/active_workers"
            ),
            1
        );
        assert!(!fs::read_dir(f.path().parent().expect("parent"))
            .expect("entries")
            .any(|e| e
                .expect("entry")
                .path()
                .extension()
                .is_some_and(|s| s == "tmp")));
    }
    #[test]
    fn cli_parsing_and_symlink_state_protection() {
        let f = Fixture::new();
        let file = f.input("plan.json", &f.plan(vec![f.order("one")]));
        assert_eq!(
            at(
                &execute(
                    &f.root,
                    &f.config,
                    "plan",
                    &["validate".into(), "--file".into(), file.into_os_string()]
                )
                .expect("validate CLI"),
                "/ok"
            ),
            true
        );
        f.create(vec![f.order("one")]);
        assert_eq!(
            at(&f.status(&f.config).expect("status"), "/queue/revision"),
            1
        );
        let backup = f.home.join("backup.json");
        fs::rename(f.path(), &backup).expect("backup");
        symlink(&backup, f.path()).expect("symlink");
        assert!(f
            .status(&f.config)
            .expect_err("unsafe queue")
            .contains("symlinks"));
        fs::remove_file(f.path()).expect("remove alias");
        fs::rename(backup, f.path()).expect("restore");
        let qdir = f.root.join(".gameskills/queues");
        let moved = f.home.join("queues-backup");
        fs::rename(&qdir, &moved).expect("move");
        symlink(&moved, &qdir).expect("directory link");
        assert!(f
            .status(&f.config)
            .expect_err("unsafe dir")
            .contains("symlinks"));
    }
    #[test]
    fn legacy_queues_are_readable_unchanged_and_cannot_cross_runtimes() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let mut legacy = f.read();
        set(&mut legacy, "schema_version", json!(1));
        legacy.as_object_mut().expect("object").remove("runtime");
        let bytes = serde_json::to_vec_pretty(&legacy).expect("legacy JSON");
        fs::write(f.path(), &bytes).expect("legacy fixture");
        let view = f.status(&f.config).expect("historical status");
        assert_eq!(at(&view, "/queue"), &legacy);
        assert_eq!(at(&view, "/compatibility/runtime"), "python");
        assert_eq!(at(&view, "/compatibility/mutable"), false);
        assert_eq!(fs::read(f.path()).expect("preserved"), bytes);
        f.unchanged("historical Python queue", || {
            f.mutate(
                "block",
                "one",
                json!({"reason":"cannot switch","checks_running":false}),
                None,
            )
        });
        let plan = changed(f.plan(vec![f.order("new")]), json!({"id":"new-wave"}));
        let file = f.input("new-plan.json", &plan);
        let args = vec!["create".into(), "--file".into(), file.into_os_string()];
        assert!(execute(&f.root, &f.config, "queue", &args)
            .expect_err("active legacy")
            .contains("unfinished orders"));
        legacy
            .pointer_mut("/orders/one/state")
            .expect("state")
            .clone_from(&json!("integrated"));
        fs::write(f.path(), serde_json::to_vec_pretty(&legacy).expect("JSON"))
            .expect("completed historical fixture");
        let original = fs::read(f.path()).expect("old bytes");
        let created = execute(&f.root, &f.config, "queue", &args)
            .expect("new Rust queue after legacy completion");
        assert_eq!(at(&created, "/queue/runtime"), "rust");
        assert_eq!(at(&created, "/queue/schema_version"), 2);
        assert_eq!(fs::read(f.path()).expect("preserved"), original);
    }
    #[test]
    fn active_legacy_added_after_rust_creation_blocks_mutation() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let mut legacy = f.read();
        set(&mut legacy, "id", json!("old"));
        set(&mut legacy, "schema_version", json!(1));
        legacy.as_object_mut().expect("object").remove("runtime");
        fs::write(
            f.root.join(".gameskills/queues/old.json"),
            serde_json::to_vec(&legacy).expect("JSON"),
        )
        .expect("legacy");
        f.unchanged("unfinished orders", || {
            f.mutate(
                "block",
                "one",
                json!({"reason":"pause","checks_running":false}),
                None,
            )
        });
    }
    #[test]
    fn duplicate_json_malformed_state_and_unsafe_hardlinks_are_rejected() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let original = fs::read(f.path()).expect("bytes");
        fs::write(f.path(), b"{\"id\":\"wave\",\"id\":\"other\"}").expect("duplicate");
        assert!(f
            .status(&f.config)
            .expect_err("duplicate")
            .contains("duplicate JSON key"));
        fs::write(f.path(), &original).expect("restore");
        let mut bad = f.read();
        bad.pointer_mut("/orders/one/state")
            .expect("state")
            .clone_from(&json!("unknown"));
        fs::write(f.path(), serde_json::to_vec(&bad).expect("JSON")).expect("invalid");
        f.unchanged("state", || {
            f.mutate(
                "block",
                "one",
                json!({"reason":"pause","checks_running":false}),
                None,
            )
        });
        fs::write(f.path(), &original).expect("restore");
        fs::hard_link(f.path(), f.home.join("hardlink")).expect("hard link");
        assert!(f
            .status(&f.config)
            .expect_err("hardlink")
            .contains("singly linked"));
    }
    #[test]
    fn unsafe_lock_modes_and_nonordinary_inputs_fail_without_writes() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let lock = f.root.join(".gameskills/queues/.lock");
        let backup = f.home.join("lock-backup");
        fs::rename(&lock, &backup).expect("move lock");
        symlink(&backup, &lock).expect("lock alias");
        f.unchanged("symlinks", || {
            f.mutate(
                "block",
                "one",
                json!({"reason":"pause","checks_running":false}),
                None,
            )
        });
        fs::remove_file(&lock).expect("remove link");
        fs::rename(backup, &lock).expect("restore lock");
        fs::set_permissions(&lock, fs::Permissions::from_mode(0o666)).expect("unsafe mode");
        assert!(f
            .status(&f.config)
            .expect_err("unsafe mode")
            .contains("without group/world writes"));
        let file = f.input("input.json", &f.plan(vec![f.order("one")]));
        let alias = f.home.join("input-alias");
        symlink(&file, &alias).expect("input alias");
        assert!(execute(
            &f.root,
            &f.config,
            "plan",
            &["validate".into(), "--file".into(), alias.into_os_string()]
        )
        .expect_err("symlink input")
        .contains("symlinks"));
        assert!(execute(
            &f.root,
            &f.config,
            "plan",
            &[
                "validate".into(),
                "--file".into(),
                f.home.clone().into_os_string()
            ]
        )
        .expect_err("directory input")
        .contains("ordinary file"));
    }
    #[test]
    fn input_rejection_and_stale_revision_do_not_modify_state() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        for args in [
            vec![
                "start",
                "wave",
                "one",
                "--expected-revision",
                "-1",
                "--worktree",
                "/tmp",
            ],
            vec![
                "start",
                "wave",
                "one",
                "--expected-revision",
                "0",
                "--worktree",
                "/tmp",
            ],
            vec!["status", "../wave"],
            vec!["inject", "wave", "--file", "missing"],
        ] {
            let args = args.into_iter().map(OsString::from).collect::<Vec<_>>();
            let before = fs::read(f.path()).expect("bytes");
            assert!(execute(&f.root, &f.config, "queue", &args).is_err());
            assert_eq!(fs::read(f.path()).expect("preserved"), before);
        }
    }
    #[test]
    fn queue_mutation_child_probe() {
        let Ok(root) = std::env::var("GAMESKILLS_WORKFLOW_CHILD_ROOT") else {
            return;
        };
        let config: Value = serde_json::from_str(
            &std::env::var("GAMESKILLS_WORKFLOW_CHILD_CONFIG").expect("child config"),
        )
        .expect("JSON config");
        let file = std::env::var("GAMESKILLS_WORKFLOW_CHILD_PAYLOAD").expect("child payload");
        let error = execute(
            Path::new(&root),
            &config,
            "queue",
            &[
                "block".into(),
                "wave".into(),
                "one".into(),
                "--file".into(),
                file.into(),
                "--expected-revision".into(),
                "1".into(),
            ],
        )
        .expect_err("stale pre-read configuration must fail");
        assert!(
            error.contains("configuration changed after it was read"),
            "{error}"
        );
    }
    #[test]
    fn configuration_is_rechecked_after_acquiring_cross_process_lock() {
        let f = Fixture::new();
        f.create(vec![f.order("one")]);
        let before = fs::read(f.path()).expect("queue");
        let file = f.input(
            "child-block.json",
            &json!({"reason":"Pause", "checks_running":false}),
        );
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(f.root.join(".gameskills/queues/.lock"))
            .expect("lock file");
        lock.lock().expect("hold queue lock");
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "posix::queue_mutation_child_probe",
                "--nocapture",
            ])
            .env("GAMESKILLS_WORKFLOW_CHILD_ROOT", &f.root)
            .env(
                "GAMESKILLS_WORKFLOW_CHILD_CONFIG",
                serde_json::to_string(&f.config).expect("JSON config"),
            )
            .env("GAMESKILLS_WORKFLOW_CHILD_PAYLOAD", file)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("child probe");
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            child.try_wait().expect("child status").is_none(),
            "another process bypassed held queue lock"
        );
        let config = changed(
            f.config.clone(),
            json!({"dispatch":{"enabled":false,"max_workers":5}}),
        );
        fs::write(f.root.join("gameskills.toml"), config_text(&config))
            .expect("setup changed configuration under queue lock");
        drop(lock);
        let output = child.wait_with_output().expect("child finished");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(f.path()).expect("preserved queue"), before);
    }
    #[test]
    fn create_rejects_stale_or_missing_configuration_without_queue_file() {
        let f = Fixture::new();
        let plan = f.input("plan.json", &f.plan(vec![f.order("one")]));
        let args = vec!["create".into(), "--file".into(), plan.into_os_string()];
        let config = changed(
            f.config.clone(),
            json!({"dispatch":{"enabled":false,"max_workers":5}}),
        );
        fs::write(f.root.join("gameskills.toml"), config_text(&config)).expect("new config");
        assert!(execute(&f.root, &f.config, "queue", &args)
            .expect_err("stale config")
            .contains("configuration changed after it was read"));
        assert!(!f.path().exists());
        fs::remove_file(f.root.join("gameskills.toml")).expect("deleted configuration");
        assert!(execute(&f.root, &f.config, "queue", &args).is_err());
        assert!(!f.path().exists());
    }

    #[test]
    fn interrupted_setup_after_readiness_cannot_create_or_mutate_a_queue() {
        let f = Fixture::new();
        let plan = f.input("plan.json", &f.plan(vec![f.order("one")]));
        let args = vec!["create".into(), "--file".into(), plan.into_os_string()];
        fs::create_dir_all(f.root.join(".gameskills")).expect("state");
        let journal = f.root.join(".gameskills/setup-transaction.json");
        fs::write(&journal, "{}").expect("interrupted transaction");
        assert!(execute(&f.root, &f.config, "queue", &args)
            .expect_err("readiness was invalidated")
            .contains("interrupted setup"));
        assert!(!f.path().exists());
        fs::remove_file(&journal).expect("recovered transaction");
        execute(&f.root, &f.config, "queue", &args).expect("create after recovery");
        fs::write(&journal, "{}").expect("later interrupted transaction");
        f.unchanged("interrupted setup", || {
            f.mutate(
                "block",
                "one",
                json!({"reason":"bounded review", "checks_running":false}),
                None,
            )
        });
        assert_eq!(
            fs::read_to_string(journal).expect("preserved journal"),
            "{}"
        );
    }
}
#[cfg(windows)]
#[test]
fn windows_reports_queue_mutation_support_limit() {
    let f = Fixture::new();
    let file = f.input("plan.json", &f.plan(vec![f.order("one")]));
    assert!(execute(
        &f.root,
        &f.config,
        "queue",
        &["create".into(), "--file".into(), file.into_os_string()]
    )
    .expect_err("unsupported queue")
    .contains("Windows is unsupported"));
}

//! Provider-level regressions: no network or real Linear mutations.
use gameskills_linear::{
    cleanup::{self, Options},
    provider::{self, Provider},
    Config,
};
use serde_json::{json, Value};
use std::{error::Error, path::Path};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
const WORKSPACE: &str = "11111111-1111-4111-8111-111111111111";
const TEAM: &str = "22222222-2222-4222-8222-222222222222";
const PROJECT: &str = "33333333-3333-4333-8333-333333333333";
const ID: &str = "44444444-4444-4444-8444-444444444444";
fn config() -> Config {
    Config {
        workspace: WORKSPACE.into(),
        team: TEAM.into(),
        project: PROJECT.into(),
        key_env: "UNSET_TEST_KEY".into(),
        export_dir: None,
        retention_days: 30,
        keep_projects: vec![],
        routes: Default::default(),
    }
}
fn ticket() -> Value {
    json!({"id":ID,"identifier":"HEX-1","title":"Completed work","description":"Preserved decisions","project":{"id":PROJECT},"team":{"id":TEAM},"state":{"id":"done","type":"completed"},"completedAt":"2026-08-01T12:00:00Z","updatedAt":"2026-09-12T00:00:00Z","trashed":false,"canceledAt":null,"parent":null,"children":[],"relations":[],"inverseRelations":[],"comments":[{"id":"comment","body":"Decision record"}],"attachments":[],"documents":[],"history":[{"id":"history","createdAt":"2026-08-01T12:00:00Z","toStateId":"done","toState":{"type":"completed"}}]})
}
fn opts(path: Option<&Path>) -> Options<'_> {
    Options {
        project: PROJECT,
        retention_days: 30,
        limit: 3,
        apply: path.is_some(),
        export_dir: path,
        now: OffsetDateTime::parse("2026-09-12T12:00:00Z", &Rfc3339).expect("constant timestamp"),
    }
}
struct Fake {
    issue: Value,
    deleted: usize,
    reads: usize,
    change: bool,
    fail_delete: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            issue: ticket(),
            deleted: 0,
            reads: 0,
            change: false,
            fail_delete: false,
        }
    }
}
fn page(nodes: Value) -> Value {
    json!({"nodes":nodes,"pageInfo":{"hasNextPage":false,"endCursor":null}})
}
impl Provider for Fake {
    fn query(&mut self, q: &str, _: Value) -> Result<Value, String> {
        if q.contains("organization{") {
            return Ok(
                json!({"organization":{"id":WORKSPACE},"team":{"id":TEAM},"project":{"id":PROJECT}}),
            );
        }
        if q.contains("issues(first") {
            return Ok(
                json!({"issues":page(if self.deleted>0 {json!([])}else{json!([{"id":ID,"identifier":"HEX-1","completedAt":self.issue.get("completedAt")}])})}),
            );
        }
        if q.starts_with("mutation") {
            assert!(q.contains("permanentlyDelete:false"));
            if self.fail_delete {
                return Err("ambiguous response".into());
            }
            self.deleted += 1;
            return Ok(json!({"issueDelete":{"success":true,"lastSyncId":7}}));
        }
        for c in [
            "children",
            "relations",
            "inverseRelations",
            "comments",
            "attachments",
            "documents",
            "history",
        ] {
            if q.contains(&format!("{c}(first:")) {
                return Ok(
                    json!({"issue":{c:page(self.issue.get(c).cloned().ok_or("missing fixture connection")?)}}),
                );
            }
        }
        self.reads += 1;
        if self.change && self.reads == 2 {
            set(
                &mut self.issue,
                "/description",
                json!("Changed after export"),
            );
        }
        Ok(json!({"issue":self.issue}))
    }
}
#[test]
fn retention_uses_completion_and_reopening_resets_it() -> Result<(), Box<dyn Error>> {
    let c = config();
    let mut o = opts(None);
    let mut v = ticket();
    o.now = OffsetDateTime::parse("2026-08-31T11:59:59Z", &Rfc3339)?;
    assert!(cleanup::eligibility(&v, &c, &o).is_err());
    o.now += time::Duration::seconds(1);
    assert!(cleanup::eligibility(&v, &c, &o).is_ok());
    set(&mut v, "/updatedAt", json!("2026-08-31T12:00:00Z"));
    assert!(cleanup::eligibility(&v, &c, &o).is_ok());
    set(&mut v, "/state/type", json!("started"));
    assert!(cleanup::eligibility(&v, &c, &o).is_err());
    v = ticket();
    set(&mut v, "/history/0/toState/type", json!("started"));
    assert!(cleanup::eligibility(&v, &c, &o).is_err());
    v = ticket();
    set(&mut v, "/completedAt", json!("2026-08-31T12:00:00Z"));
    assert!(cleanup::eligibility(&v, &c, &o).is_err());
    Ok(())
}
#[test]
fn uncertain_or_active_relationships_and_wrong_scope_are_retained() {
    let c = config();
    let o = opts(None);
    for path in ["/children", "/relations", "/inverseRelations", "/history"] {
        let mut v = ticket();
        *v.pointer_mut(path).expect("fixture field") = Value::Null;
        assert!(cleanup::eligibility(&v, &c, &o).is_err());
    }
    for key in ["children", "relations", "inverseRelations"] {
        let mut v = ticket();
        *v.get_mut(key).expect("fixture key") = json!([{"state":{"type":"started"},"issue":{"state":{"type":"started"}},"relatedIssue":{"state":{"type":"completed"}}}]);
        assert!(cleanup::eligibility(&v, &c, &o).is_err());
    }
    let mut v = ticket();
    set(&mut v, "/parent", json!({"state":{"type":"started"}}));
    assert!(cleanup::eligibility(&v, &c, &o).is_err());
    v = ticket();
    set(&mut v, "/project/id", json!(TEAM));
    assert!(cleanup::eligibility(&v, &c, &o).is_err());
    let mut c = config();
    c.keep_projects.push(PROJECT.into());
    assert!(cleanup::eligibility(&ticket(), &c, &o).is_err());
}
#[test]
fn preview_has_no_mutations_and_unsupported_content_is_retained() -> Result<(), Box<dyn Error>> {
    let mut p = Fake::new();
    let c = config();
    let o = opts(None);
    let r = cleanup::run(&mut p, &c, &o)?;
    assert_eq!(at(&r, "/results/0/status"), "eligible");
    assert_eq!(p.deleted, 0);
    set(
        &mut p.issue,
        "/description",
        json!("![Decision](https://uploads.linear.app/private.png)"),
    );
    let r = cleanup::run(&mut p, &c, &o)?;
    assert_eq!(at(&r, "/results/0/status"), "skipped");
    assert_eq!(p.deleted, 0);
    Ok(())
}
#[cfg(unix)]
fn private() -> Result<tempfile::TempDir, Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    // Fake-provider tests exercise the production guard against system temporary
    // export roots. Keep their disposable private store under the user's directory.
    let parent = std::env::var_os("HOME").ok_or("test requires a user directory")?;
    let d = tempfile::tempdir_in(parent)?;
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700))?;
    Ok(d)
}
#[test]
#[cfg(unix)]
fn apply_exports_before_delete_and_rerun_does_not_repeat() -> Result<(), Box<dyn Error>> {
    let d = private()?;
    let path = d.path().canonicalize()?;
    let o = opts(Some(&path));
    let mut p = Fake::new();
    let r = cleanup::run(&mut p, &config(), &o)?;
    assert_eq!(p.deleted, 1);
    assert_eq!(at(&r, "/results/0/status"), "deleted");
    let filename = at(&r, "/results/0/manifest").as_str().ok_or("manifest")?;
    let export: Value = serde_json::from_slice(&std::fs::read(path.join(filename))?)?;
    assert_eq!(
        at(&export, "/content/issue/comments/0/body"),
        "Decision record"
    );
    cleanup::run(&mut p, &config(), &o)?;
    assert_eq!(p.deleted, 1);
    Ok(())
}
#[test]
#[cfg(unix)]
fn changes_and_ambiguous_delete_stop_further_mutations() -> Result<(), Box<dyn Error>> {
    for change in [true, false] {
        let d = private()?;
        let path = d.path().canonicalize()?;
        let o = opts(Some(&path));
        let mut p = Fake::new();
        p.change = change;
        p.fail_delete = !change;
        let r = cleanup::run(&mut p, &config(), &o);
        assert_eq!(p.deleted, 0);
        if change {
            assert_eq!(at(&r?, "/results/0/status"), "skipped");
        } else {
            assert!(r.is_err());
            p.fail_delete = false;
            assert!(cleanup::run(&mut p, &config(), &o).is_err());
            assert_eq!(p.deleted, 0);
            cleanup::reconcile(&mut p, &config(), PROJECT, ID, &path)?;
            cleanup::run(&mut p, &config(), &o)?;
            assert_eq!(p.deleted, 1);
        }
    }
    Ok(())
}
#[test]
#[cfg(unix)]
fn missing_or_unsafe_export_store_prevents_deletion() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    let d = private()?;
    let path = d.path().canonicalize()?;
    let mut o = opts(None);
    o.apply = true;
    let mut p = Fake::new();
    assert!(cleanup::run(&mut p, &config(), &o).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    o.export_dir = Some(&path);
    assert!(cleanup::run(&mut p, &config(), &o).is_err());
    assert_eq!(p.deleted, 0);
    Ok(())
}
#[test]
fn partial_graphql_data_is_never_success() {
    assert!(provider::decode(br#"{"data":{"issueDelete":{"success":true}},"errors":[{"extensions":{"code":"RATELIMITED"}}]}"#).is_err());
    for message in ["issue limit", "unauthorized", "unknown"] {
        assert!(provider::decode(
            json!({"errors":[{"message":message}]})
                .to_string()
                .as_bytes()
        )
        .is_err());
    }
}
#[test]
fn pagination_rejects_duplicate_ids_and_cursor_loops() {
    struct Loop;
    impl Provider for Loop {
        fn query(&mut self, _: &str, _: Value) -> Result<Value, String> {
            Ok(
                json!({"items":{"nodes":[{"id":"same"}],"pageInfo":{"hasNextPage":true,"endCursor":"same"}}}),
            )
        }
    }
    assert!(provider::pages(&mut Loop, "query", json!({}), "/items").is_err());
}

fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    v.pointer(path).expect("fixture pointer")
}
fn set(v: &mut Value, path: &str, value: Value) {
    if path.matches('/').count() == 1 {
        v.as_object_mut()
            .expect("fixture object")
            .insert(path.trim_start_matches('/').into(), value);
    } else {
        *v.pointer_mut(path).expect("fixture pointer") = value;
    }
}

#[test]
#[cfg(unix)]
fn restored_recompleted_issue_gets_a_new_period_and_preserves_old_operation(
) -> Result<(), Box<dyn Error>> {
    let d = private()?;
    let path = d.path().canonicalize()?;
    let mut o = opts(Some(&path));
    let mut p = Fake::new();
    cleanup::run(&mut p, &config(), &o)?;
    p.deleted = 0;
    let r = cleanup::run(&mut p, &config(), &o)?;
    assert_eq!(at(&r, "/results/0/status"), "skipped");
    assert_eq!(p.deleted, 0);
    set(&mut p.issue, "/completedAt", json!("2026-09-01T12:00:00Z"));
    set(
        &mut p.issue,
        "/history/0/createdAt",
        json!("2026-09-01T12:00:00Z"),
    );
    let r = cleanup::run(&mut p, &config(), &o)?;
    assert_eq!(at(&r, "/results/0/status"), "skipped");
    o.now = OffsetDateTime::parse("2026-10-01T12:00:00Z", &Rfc3339)?;
    cleanup::run(&mut p, &config(), &o)?;
    assert_eq!(p.deleted, 1);
    assert!(std::fs::read_dir(path)?
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().contains("-operation-")));
    Ok(())
}

#[test]
#[cfg(unix)]
fn system_temporary_directory_is_not_a_durable_export_store() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir()?;
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700))?;
    let path = d.path().canonicalize()?;
    let mut p = Fake::new();
    assert!(cleanup::run(&mut p, &config(), &opts(Some(&path))).is_err());
    assert_eq!(p.deleted, 0);
    Ok(())
}

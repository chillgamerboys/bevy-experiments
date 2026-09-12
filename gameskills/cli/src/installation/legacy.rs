//! Non-destructive migration from the seven-skill rendered pack.
use super::{archive, files, hash, setup, sorted_bytes};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
const SKILLS: [&str; 7] = [
    "architect-bevy-game",
    "model-turn-based-game",
    "build-bevy-ui",
    "test-bevy-game",
    "verify-bevy-ui",
    "debug-bevy-runtime",
    "review-bevy-change",
];

pub(super) fn execute(root: &Path, args: &[String]) -> Result<Value, String> {
    if args.first().map(String::as_str) != Some("import") {
        return Err("legacy supports import [--apply] [--bundle PATH] [--packages NAME ...]; rendered-pack install/sync are retired".into());
    }
    let directory = files::Directory::open(root)?;
    let original = directory.read(".bevy-gamekit/skills.json")?;
    let manifest = archive::json(&original)?;
    if !matches!(
        manifest.get("schema_version").and_then(Value::as_u64),
        Some(1 | 2)
    ) || manifest.get("clients") != Some(&json!(["codex", "claude"]))
        || manifest.get("skills") != Some(&json!(SKILLS))
    {
        return Err("unsupported legacy manifest client/skill set or schema".into());
    }
    let source = manifest
        .get("source")
        .and_then(Value::as_object)
        .ok_or("legacy source identity is missing")?;
    if source
        .get("repository")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("legacy source repository is missing".into());
    }
    let revision = source
        .get("revision")
        .and_then(Value::as_str)
        .ok_or("legacy source revision is missing")?;
    if revision.is_empty()
        || revision.trim() != revision
        || revision.chars().any(char::is_control)
        || ["head", "main", "master", "latest"].contains(&revision.to_ascii_lowercase().as_str())
    {
        return Err("legacy source revision is not an explicit historical pin".into());
    }
    let recorded_sha = source.get("resolved_sha").and_then(Value::as_str);
    if manifest.get("schema_version").and_then(Value::as_u64) == Some(2) && recorded_sha.is_none() {
        return Err("legacy schema2 source requires resolved_sha".into());
    }
    if source.contains_key("resolved_sha")
        && recorded_sha.is_none_or(|sha| {
            ![40, 64].contains(&sha.len())
                || !sha
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
    {
        return Err("legacy resolved_sha must be a full lowercase Git SHA".into());
    }
    let provenance = if recorded_sha.is_some() {
        "historical revision and full SHA recorded; source Git provenance not independently verified during import"
    } else {
        "historical schema1 has no resolved SHA; Git source provenance unavailable"
    };
    let generated = manifest
        .get("generated")
        .and_then(Value::as_object)
        .ok_or("legacy manifest has no generated hashes")?;
    if generated.is_empty() {
        return Err("legacy manifest has no generated hashes".into());
    }
    for (name, digest) in generated {
        files::relative(name)?;
        let mut parts = name.split('/');
        if !matches!(parts.next(), Some(".agents" | ".claude"))
            || !matches!(parts.next(), Some("skills" | "bevy-gamekit"))
            || parts.next().is_none()
        {
            return Err("unsafe generated path in legacy manifest".into());
        }
        if digest.as_str().is_none_or(|s| {
            s.len() != 64
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        }) {
            return Err("invalid legacy generated hash".into());
        }
    }
    let base = directory.child(".bevy-gamekit/base", false)?.tree()?;
    let expected = generated
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let actual = base
        .iter()
        .map(|(name, bytes)| (name.clone(), json!(hash(bytes))))
        .collect::<BTreeMap<_, _>>();
    if expected != actual {
        return Err("legacy pinned rendered base does not match recorded hashes; restore the original metadata before import".into());
    }
    let mut preserved_edits = Vec::new();
    for (name, bytes) in &base {
        if directory.read_optional(name)?.as_ref() != Some(bytes) {
            preserved_edits.push(name.clone());
        }
    }
    let overlays = directory.child(".bevy-gamekit/overlays", false)?.tree()?;
    let report = json!({"schema_version":2,"runtime":"rust","legacy_manifest_sha256":hash(&original),"legacy_source":source,"source_provenance":provenance,"preserved_generated_edits":preserved_edits,"preserved_overlays":overlays.keys().collect::<Vec<_>>(),"policy":"all legacy client files, bases and overlays remain untouched; review duplicate legacy skills before native activation","historical_evidence":"not converted or reusable as Rust evidence"});
    let setup_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
    let applied = setup_args.iter().any(|s| s == "--apply");
    // All validation precedes the first installation mutation.
    let installation = setup::execute(root, &setup_args)?;
    if applied {
        if directory.read(".bevy-gamekit/skills.json")? != original
            || directory.child(".bevy-gamekit/base", false)?.tree()? != base
            || directory.child(".bevy-gamekit/overlays", false)?.tree()? != overlays
        {
            return Err(
                "legacy metadata changed during import; inspect installation and retry".into(),
            );
        }
        directory.write(
            ".gameskills/legacy-import.json",
            Some(&sorted_bytes(&report)?),
        )?;
    }
    Ok(json!({"ok":true,"applied":applied,"installation":installation,"import":report}))
}

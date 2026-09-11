//! Instruction archive identity and immutable materialization.
use super::{exact_keys, files, hash, sorted_bytes, strings, LIMIT};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    io::{Cursor, Read},
    path::Path,
    process::Command,
};

/// A verified, compatible instruction snapshot and its selected packages.
#[derive(Clone, Debug)]
pub struct InstructionBundle {
    /// Full unmodified content manifest.
    pub manifest: Value,
    /// Verified catalog from the snapshot.
    pub catalog: Value,
    /// Files materialized for the selected packages.
    pub files: BTreeMap<String, Vec<u8>>,
    /// Unique package selection with core present.
    pub selected: Vec<String>,
}
impl InstructionBundle {
    /// Content digest independently bound to paths, bytes and compatibility.
    pub fn identity(&self) -> &str {
        self.manifest
            .get("content_sha256")
            .and_then(Value::as_str)
            .expect("verified identity")
    }
    /// Select core and optional packages without altering the snapshot identity.
    pub fn select(&self, packages: &[String]) -> Result<Self, String> {
        let available = self
            .catalog
            .get("packages")
            .and_then(Value::as_object)
            .ok_or("invalid catalog packages")?;
        if !packages.iter().any(|p| p == "gameskills")
            || packages.iter().collect::<BTreeSet<_>>().len() != packages.len()
            || packages.iter().any(|p| !available.contains_key(p))
        {
            return Err("select unique known packages including gameskills core".into());
        }
        let mut out = self.clone();
        out.selected = packages.to_vec();
        out.selected.sort();
        out.files.retain(|name, _| {
            name.split('/')
                .nth(1)
                .is_some_and(|p| packages.iter().any(|selected| selected == p))
        });
        for package in packages {
            if !out
                .files
                .contains_key(&format!("plugins/{package}/.codex-plugin/plugin.json"))
            {
                return Err(format!(
                    "snapshot does not contain selected package {package}"
                ));
            }
        }
        Ok(out)
    }
}

// serde_json otherwise silently accepts duplicate identity keys.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("unique-key JSON")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Unique, M::Error> {
                let mut result = serde_json::Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if result.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(Unique(Value::Object(result)))
            }
            fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Unique, S::Error> {
                let mut out = Vec::new();
                while let Some(Unique(value)) = seq.next_element()? {
                    out.push(value);
                }
                Ok(Unique(Value::Array(out)))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Unique, E> {
                Ok(Unique(json!(value)))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Unique, E> {
                Ok(Unique(json!(value)))
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Unique, E> {
                Ok(Unique(json!(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Unique, E> {
                Ok(Unique(json!(value)))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Unique, E> {
                Ok(Unique(json!(value)))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| de::Error::custom("non-finite JSON number"))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
        }
        deserializer.deserialize_any(V)
    }
}
pub(super) fn json(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice::<Unique>(bytes)
        .map(|u| u.0)
        .map_err(|e| e.to_string())
}

fn manifest(value: &Value) -> Result<(), String> {
    exact_keys(
        value,
        &[
            "format",
            "schema_version",
            "catalog_version",
            "compatibility",
            "default_packages",
            "files",
            "source_commit",
            "content_sha256",
        ],
    )?;
    if value.get("format") != Some(&json!("gameskills-instructions"))
        || value.get("schema_version") != Some(&json!(1))
    {
        return Err("unsupported instruction format/schema".into());
    }
    let compatibility = value.get("compatibility").ok_or("missing compatibility")?;
    exact_keys(
        compatibility,
        &[
            "activation",
            "bevy",
            "cli_version_range",
            "config_schema",
            "evidence_schema",
            "gamekit",
            "queue_schema",
            "schema_version",
        ],
    )?;
    for (key, expected) in [
        ("schema_version", 1),
        ("config_schema", 1),
        ("queue_schema", 2),
        ("evidence_schema", 2),
    ] {
        if compatibility.get(key) != Some(&json!(expected)) {
            return Err(format!("unsupported bundle compatibility {key}"));
        }
    }
    if compatibility.get("activation") != Some(&json!("runtime")) {
        return Err(
            "instruction bundle is preparation-only; runtime activation is not enabled".into(),
        );
    }
    let requirement = compatibility
        .get("cli_version_range")
        .and_then(Value::as_str)
        .ok_or("missing CLI compatibility")?;
    if !semver::VersionReq::parse(requirement)
        .map_err(|e| e.to_string())?
        .matches(&semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?)
    {
        return Err("bundle is incompatible with this CLI version".into());
    }
    if compatibility.get("bevy").and_then(Value::as_str) != Some("0.19")
        || compatibility.get("gamekit").and_then(Value::as_str) != Some("0.1.0")
    {
        return Err("unsupported Bevy/GameKit compatibility".into());
    }
    semver::Version::parse(
        value
            .get("catalog_version")
            .and_then(Value::as_str)
            .ok_or("missing catalog version")?,
    )
    .map_err(|e| e.to_string())?;
    let commit = value
        .get("source_commit")
        .and_then(Value::as_str)
        .ok_or("missing source commit")?;
    if ![40, 64].contains(&commit.len())
        || !commit
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("source commit must be a full lowercase SHA".into());
    }
    let mut identity = value.clone();
    let map = identity.as_object_mut().ok_or("identity must be object")?;
    map.remove("source_commit");
    map.remove("content_sha256");
    if value.get("content_sha256").and_then(Value::as_str)
        != Some(hash(&sorted_bytes(&identity)?).as_str())
    {
        return Err("bundle content identity mismatch".into());
    }
    let paths = value
        .get("files")
        .and_then(Value::as_object)
        .ok_or("manifest files must be object")?;
    if paths.is_empty() || paths.len() > 4096 {
        return Err("invalid bundle file count".into());
    }
    for (name, digest) in paths {
        instruction_path(name)?;
        if digest.as_str().is_none_or(|s| {
            s.len() != 64
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        }) {
            return Err("invalid file hash".into());
        }
    }
    Ok(())
}
fn instruction_path(name: &str) -> Result<(), String> {
    files::relative(name)?;
    let parts = name.split('/').collect::<Vec<_>>();
    if parts.first() != Some(&"plugins")
        || !parts
            .get(1)
            .is_some_and(|name| crate::config::PACKAGES.contains(name))
    {
        return Err("archive path outside canonical packages".into());
    }
    let rest = parts.iter().skip(2).copied().collect::<Vec<_>>().join("/");
    if ![".codex-plugin/plugin.json", ".claude-plugin/plugin.json"].contains(&rest.as_str())
        && !(parts.get(1) == Some(&"gameskills") && rest == "catalog.json")
        && !((rest.starts_with("skills/") || rest.starts_with("references/"))
            && rest.ends_with(".md"))
    {
        return Err(format!("unsupported instruction asset: {name}"));
    }
    Ok(())
}
fn validate_files(
    value: Value,
    files: BTreeMap<String, Vec<u8>>,
    selected: Option<Vec<String>>,
) -> Result<InstructionBundle, String> {
    manifest(&value)?;
    let catalog = json(
        files
            .get("plugins/gameskills/catalog.json")
            .ok_or("bundle lacks catalog")?,
    )?;
    if catalog.get("schema_version") != Some(&json!(1))
        || catalog.get("version") != value.get("catalog_version")
    {
        return Err("invalid or mismatched catalog".into());
    }
    let packages = catalog
        .get("packages")
        .and_then(Value::as_object)
        .ok_or("invalid package catalog")?;
    if packages
        .keys()
        .any(|name| !crate::config::PACKAGES.contains(&name.as_str()))
        || !packages.contains_key("gameskills")
    {
        return Err("unsupported catalog package identity".into());
    }
    for (package, info) in packages {
        let skills = strings(info, "skills")?;
        if skills.is_empty()
            || skills.iter().collect::<BTreeSet<_>>().len() != skills.len()
            || skills
                .iter()
                .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'))
        {
            return Err(format!("invalid catalog skills: {package}"));
        }
    }
    let selected = selected.unwrap_or_else(|| packages.keys().cloned().collect());
    let expected = value
        .get("files")
        .and_then(Value::as_object)
        .expect("validated files")
        .iter()
        .filter(|(name, _)| {
            name.split('/')
                .nth(1)
                .is_some_and(|p| selected.iter().any(|s| s == p))
        })
        .map(|(name, hash)| (name.clone(), hash.clone()))
        .collect::<BTreeMap<_, _>>();
    let actual = files
        .iter()
        .map(|(name, bytes)| (name.clone(), json!(hash(bytes))))
        .collect::<BTreeMap<_, _>>();
    if actual != expected {
        return Err("bundle package content mismatch or unrecorded files".into());
    }
    let bundle = InstructionBundle {
        manifest: value,
        catalog,
        files,
        selected: selected.clone(),
    };
    bundle.select(&selected)
}

/// Verify gzip integrity, bounded raw tar entries, identity, bytes and compatibility.
pub fn verify_archive(bytes: &[u8]) -> Result<InstructionBundle, String> {
    if bytes.len() as u64 > LIMIT {
        return Err("compressed archive exceeds bounded size".into());
    }
    let mut decoder = flate2::bufread::GzDecoder::new(Cursor::new(bytes));
    let mut decoded = Vec::new();
    decoder
        .by_ref()
        .take(LIMIT + 1)
        .read_to_end(&mut decoded)
        .map_err(|e| format!("invalid gzip archive: {e}"))?;
    if decoded.len() as u64 > LIMIT {
        return Err("decompressed archive exceeds bounded size".into());
    }
    if decoder.into_inner().position() != bytes.len() as u64 {
        return Err("trailing data after gzip archive".into());
    }
    let mut archive = tar::Archive::new(Cursor::new(&decoded));
    let mut files = BTreeMap::new();
    let mut end = 0_u64;
    for entry in archive.entries().map_err(|e| e.to_string())?.raw(true) {
        let mut entry = entry.map_err(|e| e.to_string())?;
        if !entry.header().entry_type().is_file() {
            return Err("archive contains links, directories or unsupported extensions".into());
        }
        let name = std::str::from_utf8(&entry.path_bytes())
            .map_err(|e| e.to_string())?
            .to_owned();
        files::relative(&name)?;
        if name != "bundle.json" {
            instruction_path(&name)?;
        }
        if entry.size() > LIMIT || files.len() >= 4097 {
            return Err("archive exceeds bounded limits".into());
        }
        end = entry.raw_file_position() + entry.size().div_ceil(512) * 512;
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        if files.insert(name, data).is_some() {
            return Err("duplicate archive path".into());
        }
    }
    let end = usize::try_from(end).map_err(|e| e.to_string())?;
    let trailer = decoded.get(end..).ok_or("invalid tar extent")?;
    if trailer.len() < 1024 || trailer.iter().any(|byte| *byte != 0) {
        return Err("invalid or trailing tar data".into());
    }
    let identity = json(
        &files
            .remove("bundle.json")
            .ok_or("archive lacks bundle.json")?,
    )?;
    validate_files(identity, files, None)
}
pub(super) fn embedded() -> Result<InstructionBundle, String> {
    let bundle = verify_archive(include_bytes!("../../bundle/instructions.tar.gz"))?;
    if bundle.manifest != json(include_bytes!("../../bundle/bundle.json"))? {
        return Err("embedded manifest and archive disagree".into());
    }
    Ok(bundle)
}
pub(super) fn installation_id(content: &str, selected: &[String]) -> Result<String, String> {
    if content.len() != 64
        || !content
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid content identity".into());
    }
    let mut names = selected.to_vec();
    names.sort();
    Ok(format!(
        "{content}-{}",
        hash(names.join("\n").as_bytes())
            .get(..8)
            .expect("digest length")
    ))
}
pub(super) fn marketplace(bundle: &InstructionBundle) -> String {
    format!(
        "gameskills-{}",
        installation_id(bundle.identity(), &bundle.selected).expect("verified identity")
    )
}
pub(super) fn markets(bundle: &InstructionBundle) -> (Value, Value) {
    let name = marketplace(bundle);
    let codex = bundle.selected.iter().map(|p| json!({"name":p,"source":{"source":"local","path":format!("./plugins/{p}")},"policy":{"installation":"AVAILABLE","authentication":"ON_INSTALL"},"category":"Productivity"})).collect::<Vec<_>>();
    let claude = bundle.selected.iter().map(|p| json!({"name":p,"source":format!("./plugins/{p}"),"version":bundle.manifest.get("catalog_version")})).collect::<Vec<_>>();
    (
        json!({"name":name,"interface":{"displayName":"GameSkills"},"plugins":codex}),
        json!({"name":name,"owner":{"name":"GameKit contributors"},"plugins":claude}),
    )
}
pub(super) fn materialize(
    directory: &files::Directory,
    bundle: &InstructionBundle,
) -> Result<(), String> {
    for (name, bytes) in &bundle.files {
        directory.write(name, Some(bytes))?;
    }
    directory.write("bundle.json", Some(&sorted_bytes(&bundle.manifest)?))?;
    directory.write(
        "selection.json",
        Some(&sorted_bytes(
            &json!({"schema_version":2,"runtime":"rust","packages":bundle.selected}),
        )?),
    )?;
    let (codex, claude) = markets(bundle);
    directory.write(
        ".agents/plugins/marketplace.json",
        Some(&sorted_bytes(&codex)?),
    )?;
    directory.write(
        ".claude-plugin/marketplace.json",
        Some(&sorted_bytes(&claude)?),
    )?;
    Ok(())
}
pub(super) fn read_installed(
    directory: &files::Directory,
    relative: &str,
) -> Result<InstructionBundle, String> {
    let directory = directory.child(relative, false)?;
    let mut files = directory.tree()?;
    let manifest = json(
        &files
            .remove("bundle.json")
            .ok_or("missing bundle identity")?,
    )?;
    let selection = json(
        &files
            .remove("selection.json")
            .ok_or("missing Rust bundle selection")?,
    )?;
    exact_keys(&selection, &["schema_version", "runtime", "packages"])?;
    if selection.get("schema_version") != Some(&json!(2))
        || selection.get("runtime") != Some(&json!("rust"))
    {
        return Err("unsupported bundle selection".into());
    }
    let codex = json(
        &files
            .remove(".agents/plugins/marketplace.json")
            .ok_or("missing Codex marketplace")?,
    )?;
    let claude = json(
        &files
            .remove(".claude-plugin/marketplace.json")
            .ok_or("missing Claude marketplace")?,
    )?;
    let bundle = validate_files(manifest, files, Some(strings(&selection, "packages")?))?;
    if (codex, claude) != markets(&bundle) {
        return Err("bundle marketplace disagrees with selection".into());
    }
    Ok(bundle)
}
pub(super) fn source(path: &Path) -> Result<InstructionBundle, String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let path = absolute.as_path();
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("bundle must not be a symlink".into());
    }
    let parent = files::Directory::open(path.parent().ok_or("bundle has no parent")?)?;
    let leaf = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("bundle filename must be UTF-8")?;
    if metadata.is_dir() {
        read_installed(&parent, leaf)
    } else {
        verify_archive(&parent.read(leaf)?)
    }
}
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "source Git verification failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}
pub(super) fn export(root: &Path, args: &[String]) -> Result<Value, String> {
    let mut out = None;
    let mut source = None;
    let mut revision = None;
    let mut packages = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--out" if out.is_none() => out = Some(iter.next().ok_or("--out requires a path")?),
            "--source" if source.is_none() => {
                source = Some(iter.next().ok_or("--source requires a path")?)
            }
            "--revision" if revision.is_none() => {
                revision = Some(
                    iter.next()
                        .ok_or("--revision requires a full SHA or immutable tag")?,
                )
            }
            "--packages" if packages.is_empty() => {
                while iter.peek().is_some_and(|s| !s.starts_with('-')) {
                    packages.push(iter.next().expect("peeked argument").to_owned());
                }
                if packages.is_empty() {
                    return Err("--packages requires a selection".into());
                }
            }
            _ => return Err(format!("unknown bundle argument: {arg}")),
        }
    }
    let out = out.ok_or("bundle requires --out with a new directory")?;
    let bundle = if let Some(source) = source {
        let source = root.join(source);
        let revision = revision.ok_or("source export requires --revision")?;
        if revision.is_empty()
            || ["head", "main", "master", "latest"]
                .contains(&revision.to_ascii_lowercase().as_str())
        {
            return Err("use full source commit or immutable release tag".into());
        }
        let reference = if [40, 64].contains(&revision.len())
            && revision.bytes().all(|b| b.is_ascii_hexdigit())
        {
            revision.to_owned()
        } else {
            format!("refs/tags/{revision}")
        };
        let commit = String::from_utf8(git(
            &source,
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{reference}^{{commit}}"),
            ],
        )?)
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();
        if String::from_utf8(git(&source, &["rev-parse", "HEAD"])?)
            .map_err(|e| e.to_string())?
            .trim()
            != commit
        {
            return Err("source checkout does not match selected revision".into());
        }
        for command in [
            vec![
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "HEAD",
                "--",
                "plugins",
                "tools/gameskills-cli/bundle",
            ],
            vec![
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--cached",
                "HEAD",
                "--",
                "plugins",
                "tools/gameskills-cli/bundle",
            ],
            vec![
                "ls-files",
                "--others",
                "--exclude-standard",
                "--",
                "plugins",
                "tools/gameskills-cli/bundle",
            ],
        ] {
            if !git(&source, &command)?.is_empty() {
                return Err("canonical packages have uncommitted changes".into());
            }
        }
        let bytes = git(
            &source,
            &[
                "show",
                &format!("{commit}:tools/gameskills-cli/bundle/instructions.tar.gz"),
            ],
        )?;
        let bundle = verify_archive(&bytes)?;
        let manifest = json(&git(
            &source,
            &[
                "show",
                &format!("{commit}:tools/gameskills-cli/bundle/bundle.json"),
            ],
        )?)?;
        if manifest != bundle.manifest {
            return Err("prepared source snapshot disagrees with manifest".into());
        }
        let listing = git(&source, &["ls-tree", "-rz", &commit, "--", "plugins"])?;
        let mut committed = BTreeMap::new();
        for entry in listing
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
        {
            let entry = std::str::from_utf8(entry).map_err(|e| e.to_string())?;
            let (metadata, path) = entry.split_once('\t').ok_or("invalid Git tree entry")?;
            let parts = path.split('/').collect::<Vec<_>>();
            if !parts
                .get(1)
                .is_some_and(|name| crate::config::PACKAGES.contains(name))
            {
                continue;
            }
            let instruction = parts.get(2).is_some_and(|name| {
                ["skills", "references", ".codex-plugin", ".claude-plugin"].contains(name)
            }) || path == "plugins/gameskills/catalog.json";
            if !instruction {
                continue;
            }
            instruction_path(path)?;
            if !metadata.starts_with("100644 blob ") {
                return Err("canonical instruction files must be ordinary Git blobs".into());
            }
            committed.insert(
                path.to_owned(),
                git(&source, &["cat-file", "blob", &format!("{commit}:{path}")])?,
            );
        }
        if committed != bundle.files {
            return Err("prepared snapshot differs from committed canonical instructions; regenerate it with the repository tool".into());
        }
        bundle
    } else {
        if revision.is_some() {
            return Err("--revision requires --source".into());
        }
        embedded()?
    };
    let selected = if packages.is_empty() {
        strings(&bundle.manifest, "default_packages")?
    } else {
        packages
    };
    let bundle = bundle.select(&selected)?;
    let out = root.join(out);
    if std::fs::symlink_metadata(&out).is_ok() {
        return Err("bundle destination exists; immutable bundles are never overwritten".into());
    }
    let parent = files::Directory::open(out.parent().ok_or("destination has no parent")?)?;
    let leaf = out
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid destination filename")?;
    let staging = format!(
        ".export-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    );
    let directory = parent.child(&staging, true)?;
    materialize(&directory, &bundle)?;
    read_installed(&parent, &staging)?;
    if parent.exists(leaf)? {
        return Err("bundle destination exists; immutable bundles are never overwritten".into());
    }
    parent.rename(&staging, leaf)?;
    Ok(
        json!({"ok":true,"bundle":out,"content_sha256":bundle.identity(),"source_commit":bundle.manifest.get("source_commit"),"packages":selected,"marketplace":marketplace(&bundle)}),
    )
}

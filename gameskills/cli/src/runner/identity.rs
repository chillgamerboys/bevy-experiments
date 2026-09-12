use super::{digest, graph::Spec, hash, state::Directory};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(result.stdout)
}
fn text(root: &Path, args: &[&str]) -> Result<String, String> {
    String::from_utf8(git(root, args)?)
        .map(|s| s.trim().into())
        .map_err(|e| e.to_string())
}
pub(super) fn repository(root: &Path) -> Result<Value, String> {
    let canonical = root.canonicalize().map_err(|e| e.to_string())?;
    let top = PathBuf::from(text(root, &["rev-parse", "--show-toplevel"])?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if canonical != top {
        return Err("--root must name the Git worktree root".into());
    }
    let git_dir = PathBuf::from(text(root, &["rev-parse", "--absolute-git-dir"])?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let common_dir = PathBuf::from(text(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?)
    .canonicalize()
    .map_err(|e| e.to_string())?;
    Ok(
        json!({"root": canonical, "git_dir": git_dir, "common_dir": common_dir,
        "head": text(root, &["rev-parse", "--verify", "HEAD"])?,
        "head_ref": text(root, &["rev-parse", "--symbolic-full-name", "HEAD"])?,
        "refs_digest": hash(git(root, &["for-each-ref", "--sort=refname", "--format=%(refname)%00%(objectname)%00%(symref)"])?) }),
    )
}
fn file_hash(mut file: File) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let before = file.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() {
        return Err("unsupported source type".into());
    }
    let mut hash = Sha256::new();
    let mut bytes = [0_u8; 65536];
    loop {
        let count = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(bytes.get(..count).ok_or("invalid file read length")?);
    }
    let after = file.metadata().map_err(|e| e.to_string())?;
    if (
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.len(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) {
        return Err("source changed while fingerprinting".into());
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn executable(root: &Path, spec: &Spec) -> Result<Value, String> {
    let program = spec.argv.first().ok_or("empty argv")?;
    let cwd = root.join(&spec.cwd);
    let candidate = if program.contains('/') {
        Some(cwd.join(program))
    } else {
        std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin")),
        )
        .map(|p| cwd.join(p).join(program))
        .find(|p| {
            p.metadata()
                .is_ok_and(|m| m.is_file() && m.mode() & 0o111 != 0)
        })
    };
    let resolved = candidate.as_ref().and_then(|p| p.canonicalize().ok());
    if let Some(path) = resolved.filter(|p| p.is_file()) {
        Ok(
            json!({"path": path, "sha256": file_hash(File::open(&path).map_err(|e| e.to_string())?)?}),
        )
    } else {
        Ok(json!({"path": candidate, "missing": true}))
    }
}
pub(super) fn identity(
    root: &Path,
    config: &Value,
    commands: &BTreeMap<String, Spec>,
) -> Result<Value, String> {
    let repository = repository(root)?;
    let staged = hash(git(
        root,
        &[
            "diff",
            "--cached",
            "--binary",
            "--no-ext-diff",
            "HEAD",
            "--",
            ".",
            ":(exclude).gameskills",
            ":(exclude)gameskills.toml",
            ":(exclude)gameskills.lock.json",
        ],
    )?);
    let listing = git(
        root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let mut paths: Vec<_> = listing
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .collect();
    paths.sort();
    paths.dedup();
    let mut entries = Vec::new();
    for raw in paths {
        if raw == b".gameskills"
            || raw.starts_with(b".gameskills/")
            || [b"gameskills.toml".as_slice(), b"gameskills.lock.json"].contains(&raw)
        {
            continue;
        }
        let path = PathBuf::from(OsString::from_vec(raw.to_vec()));
        let Some(parent) = Directory::source_parent(root, path.parent().unwrap_or(Path::new(".")))?
        else {
            entries.push(json!({"path_bytes":raw,"kind":"missing"}));
            continue;
        };
        let name = path.file_name().ok_or("invalid tracked path")?;
        let info = match fs::statat(&parent.0, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(info) => info,
            Err(rustix::io::Errno::NOENT) => {
                entries.push(json!({"path_bytes":raw,"kind":"missing"}));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        let mut entry = json!({"path_bytes": raw, "mode": info.st_mode & 0o7777});
        let object = entry.as_object_mut().ok_or("source entry malformed")?;
        match FileType::from_raw_mode(info.st_mode) {
            FileType::RegularFile => {
                let fd = fs::openat(
                    &parent.0,
                    name,
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| e.to_string())?;
                let actual = fs::fstat(&fd).map_err(|e| e.to_string())?;
                if (actual.st_dev, actual.st_ino) != (info.st_dev, info.st_ino) {
                    return Err("source changed while fingerprinting".into());
                }
                object.insert("kind".into(), "file".into());
                object.insert("sha256".into(), file_hash(fd.into())?.into());
            }
            FileType::Symlink => {
                object.insert("kind".into(), "symlink".into());
                object.insert(
                    "target_bytes".into(),
                    json!(fs::readlinkat(&parent.0, name, Vec::new())
                        .map_err(|e| e.to_string())?
                        .as_bytes()),
                );
            }
            FileType::Directory if root.join(&path).join(".git").exists() => {
                object.insert("kind".into(), "submodule".into());
                object.insert(
                    "identity".into(),
                    identity(&root.join(path), &json!({}), &BTreeMap::new())?,
                );
            }
            _ => return Err(format!("unsupported source file type: {}", path.display())),
        }
        entries.push(entry);
    }
    let directory = Directory::root(root)?;
    let mut managed = BTreeMap::new();
    for name in ["gameskills.toml", "gameskills.lock.json"] {
        let exists = fs::statat(&directory.0, name, AtFlags::SYMLINK_NOFOLLOW);
        let value = match exists {
            Err(rustix::io::Errno::NOENT) => Value::Null,
            Err(e) => return Err(e.to_string()),
            Ok(_) => directory.file_digest(name)?.into(),
        };
        managed.insert(name, value);
    }
    let executables = commands
        .iter()
        .map(|(name, spec)| Ok((name.clone(), executable(root, spec)?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let environment: BTreeMap<Vec<u8>, Vec<u8>> = std::env::vars_os()
        .filter(|(key, _)| {
            !["PWD", "OLDPWD", "SHLVL", "_"]
                .iter()
                .any(|ignore| key == *ignore)
        })
        .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
        .collect();
    let environment: Vec<_> = environment.into_iter().collect();
    let runtime = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(
        json!({"repository":repository,"source_digest":digest(&(staged,entries))?,"config_digest":digest(config)?,"commands_digest":digest(commands)?,"managed_files":managed,"executables":executables,
        "environment_digest":digest(&environment)?,"runtime":{"language":"rust","version":env!("CARGO_PKG_VERSION"),"executable":runtime,"sha256":file_hash(File::open(&runtime).map_err(|e|e.to_string())?)?,"os":std::env::consts::OS,"arch":std::env::consts::ARCH}}),
    )
}

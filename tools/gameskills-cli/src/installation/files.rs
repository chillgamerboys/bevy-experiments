//! Checked and descriptor-anchored project-local I/O.
use super::LIMIT;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) struct Directory {
    file: File,
    path: PathBuf,
}

static NONCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn relative(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.starts_with('/')
        || name.contains(['\\', ':', '\0'])
        || name.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || p.ends_with(['.', ' '])
                || p.chars().any(char::is_control)
        })
    {
        return Err(format!("unsafe or non-canonical path: {name}"));
    }
    Ok(())
}

impl Directory {
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        if fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("directory must not be a symlink".into());
        }
        #[cfg(unix)]
        let file: File = rustix::fs::open(
            path,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(|e| e.to_string())?
        .into();
        #[cfg(not(unix))]
        let file = {
            use std::os::windows::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .read(true)
                .custom_flags(0x02000000 | 0x00200000)
                .open(path)
                .map_err(|e| e.to_string())?
        };
        if !file.metadata().map_err(|e| e.to_string())?.is_dir() {
            return Err("expected ordinary directory".into());
        }
        Ok(Self {
            file,
            path: path.to_owned(),
        })
    }
    pub(super) fn child(&self, name: &str, create: bool) -> Result<Self, String> {
        relative(name)?;
        let mut current = Self {
            file: self.file.try_clone().map_err(|e| e.to_string())?,
            path: self.path.clone(),
        };
        for component in name.split('/') {
            let path = current.path.join(component);
            #[cfg(unix)]
            let file: File = {
                use rustix::fs::{mkdirat, openat, Mode, OFlags};
                if create {
                    match mkdirat(&current.file, component, Mode::from_raw_mode(0o700)) {
                        Ok(()) => current.file.sync_all().map_err(|e| e.to_string())?,
                        Err(rustix::io::Errno::EXIST) => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                openat(
                    &current.file,
                    component,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| format!("invalid ordinary directory {name}: {e}"))?
                .into()
            };
            #[cfg(not(unix))]
            let file = {
                if create {
                    match fs::create_dir(&path) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                Self::open(&path)?.file
            };
            current = Self { file, path };
        }
        Ok(current)
    }
    fn parent(&self, name: &str, create: bool) -> Result<(Self, String), String> {
        relative(name)?;
        match name.rsplit_once('/') {
            Some((parent, leaf)) => Ok((self.child(parent, create)?, leaf.into())),
            None => Ok((
                Self {
                    file: self.file.try_clone().map_err(|e| e.to_string())?,
                    path: self.path.clone(),
                },
                name.into(),
            )),
        }
    }
    fn opened(&self, leaf: &str, create: bool) -> Result<File, std::io::Error> {
        #[cfg(unix)]
        let file: File = {
            use rustix::fs::{openat, Mode, OFlags};
            let flags = OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC
                | if create {
                    OFlags::RDWR | OFlags::CREATE
                } else {
                    OFlags::RDONLY
                };
            openat(&self.file, leaf, flags, Mode::from_raw_mode(0o600))?.into()
        };
        #[cfg(not(unix))]
        let file = {
            let path = self.path.join(leaf);
            if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(std::io::Error::other("file must not be a symlink"));
            }
            fs::OpenOptions::new()
                .read(true)
                .write(create)
                .create(create)
                .open(path)?
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(std::io::Error::other("expected ordinary file"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 1 {
                return Err(std::io::Error::other("file must not have hard links"));
            }
        }
        Ok(file)
    }
    pub(super) fn read_optional(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
        let (parent, leaf) = match self.parent(name, false) {
            Ok(value) => value,
            Err(error) => {
                // Missing parents are distinct from unsafe parents.
                let mut path = self.path.clone();
                for part in name.split('/') {
                    path.push(part);
                    match fs::symlink_metadata(&path) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                        Err(_) => break,
                        Ok(m) if m.file_type().is_symlink() => return Err(error),
                        Ok(_) => {}
                    }
                }
                return Err(error);
            }
        };
        let file = match parent.opened(&leaf, false) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("cannot read {name}: {e}")),
        };
        if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
            return Err("file exceeds bounded size".into());
        }
        let mut bytes = Vec::new();
        file.take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > LIMIT {
            return Err("file exceeds bounded size".into());
        }
        Ok(Some(bytes))
    }
    pub(super) fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        self.read_optional(name)?
            .ok_or_else(|| format!("missing file: {name}"))
    }
    pub(super) fn text(&self, name: &str) -> Result<String, String> {
        String::from_utf8(self.read(name)?).map_err(|e| e.to_string())
    }
    pub(super) fn write(&self, name: &str, bytes: Option<&[u8]>) -> Result<(), String> {
        let (parent, leaf) = self.parent(name, true)?;
        parent.read_optional(&leaf)?;
        let Some(bytes) = bytes else {
            return parent.remove(&leaf);
        };
        let temporary = format!(
            ".gameskills-{}-{}.tmp",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        );
        #[cfg(unix)]
        let mut file: File = rustix::fs::openat(
            &parent.file,
            &temporary,
            rustix::fs::OFlags::WRONLY
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::EXCL
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::from_raw_mode(0o600),
        )
        .map_err(|e| e.to_string())?
        .into();
        #[cfg(not(unix))]
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(parent.path.join(&temporary))
            .map_err(|e| e.to_string())?;
        let result = (|| {
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|e| e.to_string())?;
            parent.rename(&temporary, &leaf)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = parent.remove(&temporary);
        }
        result
    }
    pub(super) fn rename(&self, old: &str, new: &str) -> Result<(), String> {
        let (source, old) = self.parent(old, false)?;
        let (destination, new) = self.parent(new, false)?;
        #[cfg(unix)]
        {
            rustix::fs::renameat(&source.file, old, &destination.file, new)
                .map_err(|e| e.to_string())?;
            source.file.sync_all().map_err(|e| e.to_string())?;
            destination.file.sync_all().map_err(|e| e.to_string())
        }
        #[cfg(not(unix))]
        {
            fs::rename(source.path.join(old), destination.path.join(new)).map_err(|e| e.to_string())
        }
    }
    pub(super) fn remove(&self, leaf: &str) -> Result<(), String> {
        let (parent, leaf) = self.parent(leaf, false)?;
        #[cfg(unix)]
        let result: Result<(), std::io::Error> =
            rustix::fs::unlinkat(&parent.file, leaf, rustix::fs::AtFlags::empty())
                .map_err(Into::into);
        #[cfg(not(unix))]
        let result = fs::remove_file(parent.path.join(leaf));
        match result {
            Ok(()) => {
                #[cfg(unix)]
                parent.file.sync_all().map_err(|e| e.to_string())?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub(super) fn lock(&self, name: &str) -> Result<File, String> {
        let (parent, leaf) = self.parent(name, true)?;
        let file = parent.opened(&leaf, true).map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|e| format!("another setup is running: {e}"))?;
        Ok(file)
    }
    pub(super) fn names(&self) -> Result<Vec<String>, String> {
        fs::read_dir(&self.path)
            .map_err(|e| e.to_string())?
            .map(|entry| {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "non-UTF8 filename")?;
                relative(&name)?;
                Ok(name)
            })
            .collect()
    }
    pub(super) fn exists(&self, name: &str) -> Result<bool, String> {
        relative(name)?;
        match fs::symlink_metadata(self.path.join(name)) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.to_string()),
        }
    }
    pub(super) fn tree(&self) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
        fn walk(
            anchor: &Directory,
            directory: &Directory,
            prefix: &str,
            out: &mut std::collections::BTreeMap<String, Vec<u8>>,
            total: &mut u64,
        ) -> Result<(), String> {
            for entry in fs::read_dir(&directory.path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "non-UTF8 file name")?;
                relative(&name)?;
                let relative = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}/{name}")
                };
                let kind = entry.file_type().map_err(|e| e.to_string())?;
                if kind.is_symlink() {
                    return Err("tree contains a symlink".into());
                }
                if kind.is_dir() {
                    walk(
                        anchor,
                        &directory.child(&name, false)?,
                        &relative,
                        out,
                        total,
                    )?;
                } else {
                    let data = anchor.read(&relative)?;
                    *total += data.len() as u64;
                    if *total > LIMIT {
                        return Err("tree exceeds bounded size".into());
                    }
                    out.insert(relative, data);
                }
            }
            Ok(())
        }
        let mut out = std::collections::BTreeMap::new();
        walk(self, self, "", &mut out, &mut 0)?;
        Ok(out)
    }
}

#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn nested_removal_does_not_follow_a_symbolic_parent() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let victim = outside.path().join("journal.json");
        std::fs::write(&victim, "owned outside the installation")?;
        std::os::unix::fs::symlink(outside.path(), root.path().join("alias"))?;
        let directory = super::Directory::open(root.path())?;
        assert!(directory.remove("alias/journal.json").is_err());
        assert_eq!(
            std::fs::read_to_string(&victim)?,
            "owned outside the installation"
        );
        directory.child("state", true)?;
        directory.write("state/journal.json", Some(b"completed"))?;
        directory.remove("state/journal.json")?;
        assert!(!root.path().join("state/journal.json").exists());
        Ok(())
    }
}

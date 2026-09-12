//! Descriptor-anchored state; no path reopening during writes.
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::path::{Component, Path};
pub(crate) struct Directory(pub(crate) OwnedFd);
pub(crate) fn ordinary(fd: impl AsFd) -> Result<(), String> {
    let info = fs::fstat(fd).map_err(|e| e.to_string())?;
    if FileType::from_raw_mode(info.st_mode) != FileType::RegularFile
        || info.st_nlink != 1
        || info.st_uid != rustix::process::getuid().as_raw()
        || info.st_mode & 0o022 != 0
    {
        return Err("unsafe state file: expected private ordinary file with one link".into());
    }
    Ok(())
}
impl Directory {
    pub(crate) fn root(path: &Path) -> Result<Self, String> {
        fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(Self)
        .map_err(|e| format!("open directory {}: {e}", path.display()))
    }
    pub(crate) fn checked(fd: OwnedFd, private: bool) -> Result<Self, String> {
        let info = fs::fstat(&fd).map_err(|e| e.to_string())?;
        if FileType::from_raw_mode(info.st_mode) != FileType::Directory
            || private
                && (info.st_uid != rustix::process::getuid().as_raw() || info.st_mode & 0o022 != 0)
        {
            return Err("unsafe state directory ownership, mode, or type".into());
        }
        Ok(Self(fd))
    }
    pub(crate) fn name(name: &std::ffi::OsStr) -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        let raw = name.as_bytes();
        if raw.is_empty()
            || raw == b"."
            || raw == b".."
            || raw.iter().any(|b| [b'/', b'\\', 0].contains(b))
        {
            return Err("unsafe state filename".into());
        }
        Ok(())
    }
    pub(crate) fn child(&self, name: &str, create: bool, exclusive: bool) -> Result<Self, String> {
        Self::name(name.as_ref())?;
        if create {
            match fs::mkdirat(&self.0, name, Mode::RWXU) {
                Ok(()) => (),
                Err(rustix::io::Errno::EXIST) if !exclusive => (),
                Err(e) => return Err(format!("create state directory {name}: {e}")),
            }
        }
        let fd = fs::openat(
            &self.0,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| format!("open state directory {name}: {e}"))?;
        Self::checked(fd, true)
    }
    pub(crate) fn cwd(root: &Path, path: &Path) -> Result<Self, String> {
        Self::source_parent(root, path)?
            .ok_or_else(|| format!("missing command directory {}", path.display()))
    }
    pub(crate) fn source_parent(root: &Path, path: &Path) -> Result<Option<Self>, String> {
        let mut directory = Self::root(root)?;
        for part in path.components() {
            match part {
                Component::CurDir => (),
                Component::Normal(name) if name != ".git" && name != ".gameskills" => {
                    let fd = match fs::openat(
                        &directory.0,
                        name,
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    ) {
                        Ok(fd) => fd,
                        Err(rustix::io::Errno::NOENT) => return Ok(None),
                        Err(e) => {
                            return Err(format!("command/source directory {}: {e}", path.display()))
                        }
                    };
                    directory = Self::checked(fd, false)?;
                }
                _ => {
                    return Err(
                        "command cwd must stay inside the repository, outside metadata".into(),
                    )
                }
            }
        }
        Ok(Some(directory))
    }
    pub(crate) fn open(&self, name: &str, create: bool, exclusive: bool) -> Result<File, String> {
        Self::name(name.as_ref())?;
        let flags = OFlags::NOFOLLOW
            | OFlags::NONBLOCK
            | OFlags::CLOEXEC
            | if create { OFlags::RDWR } else { OFlags::RDONLY };
        let fd = if create {
            match fs::openat(
                &self.0,
                name,
                flags | OFlags::CREATE | OFlags::EXCL,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(fd) => fd,
                Err(rustix::io::Errno::EXIST) if !exclusive => {
                    fs::openat(&self.0, name, flags, Mode::empty())
                        .map_err(|e| format!("open {name}: {e}"))?
                }
                Err(e) => return Err(format!("create {name}: {e}")),
            }
        } else {
            fs::openat(&self.0, name, flags, Mode::empty())
                .map_err(|e| format!("open {name}: {e}"))?
        };
        ordinary(&fd)?;
        Ok(fd.into())
    }
    pub(crate) fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        self.open(name, false, false)?
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Ok(bytes)
    }
    pub(crate) fn file_digest(&self, name: &str) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        let mut file = self.open(name, false, false)?;
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 65536];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            digest.update(buffer.get(..count).ok_or("invalid log read length")?);
        }
        Ok(format!("{:x}", digest.finalize()))
    }
    pub(crate) fn write_json(
        &self,
        name: &str,
        value: &impl serde::Serialize,
    ) -> Result<(), String> {
        Self::name(name.as_ref())?;
        let temporary = format!(".write-{}", super::identifier());
        let result = (|| {
            let mut file = self.open(&temporary, true, true)?;
            serde_json::to_writer(&mut file, value).map_err(|e| e.to_string())?;
            file.write_all(b"\n")
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            fs::renameat(&self.0, &temporary, &self.0, name).map_err(|e| e.to_string())?;
            fs::fsync(&self.0).map_err(|e| e.to_string())
        })();
        let _ = fs::unlinkat(&self.0, &temporary, AtFlags::empty());
        result
    }
    pub(crate) fn entries(&self) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        let mut entries = fs::Dir::read_from(&self.0).map_err(|e| e.to_string())?;
        for entry in &mut entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if let Ok(name) = entry.file_name().to_str() {
                names.push(name.into());
            }
        }
        names.sort();
        Ok(names)
    }
}
pub(crate) fn lock(file: &File) -> Result<bool, String> {
    match fs::flock(file, fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(true),
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}
pub(crate) struct Resources {
    directory: Directory,
    common: String,
}
fn resource_digest(scope: &str, resource: &str) -> Result<String, String> {
    // Share lock filenames with historical Python runners, including Unicode.
    let serialized = serde_json::to_string(&[scope, resource]).map_err(|e| e.to_string())?;
    let mut ascii = String::new();
    for character in serialized.chars() {
        if character.is_ascii() {
            ascii.push(character);
        } else {
            let mut units = [0_u16; 2];
            for unit in character.encode_utf16(&mut units) {
                use std::fmt::Write;
                write!(ascii, "\\u{unit:04x}").map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(super::hash(ascii))
}
impl Resources {
    pub(crate) fn new(common: &str) -> Result<Self, String> {
        let directory = Directory::root(
            &Path::new("/tmp")
                .canonicalize()
                .map_err(|e| e.to_string())?,
        )?
        .child(
            &format!(
                "gameskills-resources-{}",
                rustix::process::getuid().as_raw()
            ),
            true,
            false,
        )?;
        Ok(Self {
            directory,
            common: common.into(),
        })
    }
    pub(crate) fn acquire(&self, resources: &[String]) -> Result<Option<Vec<File>>, String> {
        let mut held = Vec::new();
        for resource in resources {
            let scope = if resource.starts_with("project:") {
                self.common.as_str()
            } else {
                "global"
            };
            let name = format!("{}.lock", resource_digest(scope, resource)?);
            let file = self.directory.open(&name, true, false)?;
            if !lock(&file)? {
                return Ok(None);
            }
            held.push(file);
        }
        Ok(Some(held))
    }
}

#[cfg(test)]
mod tests {
    use super::Directory;
    #[test]
    fn unicode_resources_keep_historical_lock_names() -> Result<(), String> {
        assert_eq!(
            super::resource_digest("global", "港🎮")?,
            crate::runner::hash(r#"["global","\u6e2f\ud83c\udfae"]"#)
        );
        Ok(())
    }
    #[test]
    fn swapped_state_directory_does_not_redirect_atomic_record_writes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scratch = tempfile::tempdir()?;
        let root = Directory::root(scratch.path())?;
        let directory = root.child("anchored", true, true)?;
        std::fs::create_dir(scratch.path().join("elsewhere"))?;
        std::fs::rename(
            scratch.path().join("anchored"),
            scratch.path().join("moved"),
        )?;
        std::os::unix::fs::symlink("elsewhere", scratch.path().join("anchored"))?;
        directory.write_json(
            "record.json",
            &serde_json::json!({"observation":"anchored"}),
        )?;
        assert!(scratch.path().join("moved/record.json").is_file());
        assert!(!scratch.path().join("elsewhere/record.json").exists());
        Ok(())
    }
}

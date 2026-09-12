//! Private descriptor-anchored export storage, atomic writes and one sweep lock.
#[cfg(unix)]
mod posix {
    use rustix::fs::{self, Mode, OFlags};
    use std::{
        fs::File,
        io::{Read, Write},
        os::fd::OwnedFd,
        path::{Component, Path},
    };
    pub(crate) struct Store {
        directory: OwnedFd,
        _lock: File,
    }
    impl Drop for Store {
        fn drop(&mut self) {
            // CLOEXEC closes inherited descriptors at exec, not during the fork
            // window. Release this open-file-description lock explicitly so an
            // unrelated spawning thread cannot prolong it across a retry.
            let _ = fs::flock(&self._lock, fs::FlockOperation::Unlock);
        }
    }
    impl Store {
        pub(crate) fn open(path: &Path) -> Result<Self, String> {
            if !path.is_absolute() {
                return Err("export directory must be an absolute backed-up private path".into());
            }
            // Reject symlinks in every component, including ancestor aliases.
            let mut directory = fs::open(
                "/",
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| e.to_string())?;
            for component in path.components() {
                match component {
                    Component::RootDir => {}
                    Component::Normal(name) => {
                        directory = fs::openat(
                            &directory,
                            name,
                            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                            Mode::empty(),
                        )
                        .map_err(|_| "export directory must exist without symlink components")?;
                    }
                    _ => return Err("invalid export path".into()),
                }
            }
            let stat = fs::fstat(&directory).map_err(|e| e.to_string())?;
            if stat.st_uid != rustix::process::getuid().as_raw() || stat.st_mode & 0o077 != 0 {
                return Err(
                    "export directory must be owned by the current user with mode 0700".into(),
                );
            }
            let lock = Self::file(&directory, ".sweep.lock", true)?;
            fs::flock(&lock, fs::FlockOperation::NonBlockingLockExclusive).map_err(|e| {
                format!("cannot acquire export store lock (another sweep may be active): {e}")
            })?;
            Ok(Self {
                directory,
                _lock: lock,
            })
        }
        fn file(directory: &OwnedFd, name: &str, create: bool) -> Result<File, String> {
            if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
                return Err("invalid export filename".into());
            }
            let flags = OFlags::NOFOLLOW
                | OFlags::CLOEXEC
                | OFlags::NONBLOCK
                | if create {
                    OFlags::RDWR | OFlags::CREATE
                } else {
                    OFlags::RDONLY
                };
            let fd = fs::openat(directory, name, flags, Mode::RUSR | Mode::WUSR)
                .map_err(|e| e.to_string())?;
            let stat = fs::fstat(&fd).map_err(|e| e.to_string())?;
            if fs::FileType::from_raw_mode(stat.st_mode) != fs::FileType::RegularFile
                || stat.st_nlink != 1
                || stat.st_uid != rustix::process::getuid().as_raw()
                || stat.st_mode & 0o077 != 0
            {
                return Err("unsafe export file".into());
            }
            Ok(fd.into())
        }
        pub(crate) fn read(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
            match fs::statat(&self.directory, name, fs::AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => return Ok(None),
                Err(e) => return Err(e.to_string()),
                Ok(_) => {}
            }
            let mut out = Vec::new();
            Self::file(&self.directory, name, false)?
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut out)
                .map_err(|e| e.to_string())?;
            if out.len() > 64 * 1024 * 1024 {
                return Err("export exceeds size bound".into());
            }
            Ok(Some(out))
        }
        pub(crate) fn write(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
            // Lock serializes writers. A pre-existing unsafe temp file blocks, never follows it.
            let temp = ".sweep-write";
            let mut file = Self::file(&self.directory, temp, true)?;
            file.set_len(0)
                .and_then(|_| file.write_all(bytes))
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            fs::renameat(&self.directory, temp, &self.directory, name)
                .map_err(|e| e.to_string())?;
            fs::fsync(&self.directory).map_err(|e| e.to_string())?;
            if self.read(name)?.as_deref() != Some(bytes) {
                return Err("export readback mismatch".into());
            }
            Ok(())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn release_does_not_wait_for_a_duplicated_descriptor(
        ) -> Result<(), Box<dyn std::error::Error>> {
            use std::os::unix::fs::PermissionsExt;
            let root = tempfile::tempdir()?;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))?;
            let path = root.path().canonicalize()?;
            let store = Store::open(&path)?;
            // dup models the same open-file description inherited between fork
            // and exec, without unsafe fork inside a threaded Rust test harness.
            let inherited = store._lock.try_clone()?;
            assert!(Store::open(&path).is_err());
            drop(store);
            let next = Store::open(&path)?;
            drop(inherited);
            drop(next);
            Ok(())
        }
    }
}
#[cfg(unix)]
pub(crate) use posix::Store;
#[cfg(not(unix))]
pub(crate) struct Store;
#[cfg(not(unix))]
impl Store {
    pub(crate) fn open(_: &std::path::Path) -> Result<Self, String> {
        Err("cleanup apply requires POSIX private storage".into())
    }
    pub(crate) fn read(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
        Err("unsupported private storage".into())
    }
    pub(crate) fn write(&self, _: &str, _: &[u8]) -> Result<(), String> {
        Err("unsupported private storage".into())
    }
}

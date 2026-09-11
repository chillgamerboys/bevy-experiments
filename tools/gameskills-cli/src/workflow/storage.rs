//! Locked, descriptor-anchored state IO and strict bounded JSON input.
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Value};
use std::{fmt, io::Read, path::Path};
const LIMIT: u64 = 16 * 1024 * 1024;
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Json;
        impl<'de> Visitor<'de> for Json {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON value without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_none<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
                let mut values = vec![];
                while let Some(Strict(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(Strict(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
                let mut values = Map::new();
                while let Some((key, Strict(v))) = map.next_entry::<String, Strict>()? {
                    if values.insert(key.clone(), v).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key {key}")));
                    }
                }
                Ok(Strict(Value::Object(values)))
            }
        }
        d.deserialize_any(Json)
    }
}
fn read_json(file: std::fs::File) -> Result<Value, String> {
    let mut bytes = vec![];
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("queue or input JSON exceeds 16 MiB limit".into());
    }
    serde_json::from_slice::<Strict>(&bytes)
        .map(|v| v.0)
        .map_err(|e| format!("cannot read JSON: {e}"))
}
fn open_input(path: &Path) -> Result<std::fs::File, String> {
    #[cfg(unix)]
    {
        use rustix::fs::{open, Mode, OFlags};
        let fd = open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| format!("cannot read ordinary input file (symlinks are refused): {e}"))?;
        let file = std::fs::File::from(fd);
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("input must be an ordinary file".into());
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("input must be an ordinary file without symlinks".into());
        }
        std::fs::File::open(path).map_err(|e| e.to_string())
    }
}
pub(super) fn read_input(path: &Path) -> Result<Value, String> {
    read_json(open_input(path)?)
}
pub(super) fn current_configuration(root: &Path) -> Result<Value, String> {
    let mut source = String::new();
    open_input(&root.join("gameskills.toml"))?
        .take(LIMIT + 1)
        .read_to_string(&mut source)
        .map_err(|e| e.to_string())?;
    if source.len() as u64 > LIMIT {
        return Err("configuration exceeds 16 MiB limit".into());
    }
    serde_json::to_value(crate::config::parse(&source)?).map_err(|e| e.to_string())
}
#[cfg(unix)]
mod posix {
    use super::*;
    use rustix::fs::{self, AtFlags, Mode, OFlags};
    use std::{
        fs::File,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    pub(in super::super) struct Directory {
        directory: File,
        _lock: File,
    }
    fn io(error: impl fmt::Display) -> String {
        format!("queue state IO failed (symlinks and unsafe files are refused): {error}")
    }
    fn ordinary(file: &File) -> Result<(), String> {
        let stat = fs::fstat(file).map_err(io)?;
        if fs::FileType::from_raw_mode(stat.st_mode) != fs::FileType::RegularFile
            || stat.st_nlink != 1
            || stat.st_uid != rustix::process::getuid().as_raw()
            || stat.st_mode & 0o022 != 0
        {
            return Err("queue state files must be singly linked ordinary files owned by this user without group/world writes".into());
        }
        Ok(())
    }
    fn directory(parent: &File, name: &str, create: bool) -> Result<File, String> {
        if create {
            match fs::mkdirat(parent, name, Mode::RUSR | Mode::WUSR | Mode::XUSR) {
                Ok(()) => {}
                Err(e) if e == rustix::io::Errno::EXIST => {}
                Err(e) => return Err(io(e)),
            }
        }
        let file = File::from(
            fs::openat(
                parent,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(io)?,
        );
        let stat = fs::fstat(&file).map_err(io)?;
        if stat.st_uid != rustix::process::getuid().as_raw() || stat.st_mode & 0o022 != 0 {
            return Err(
                "queue state directories must be owned by this user without group/world writes"
                    .into(),
            );
        }
        Ok(file)
    }
    impl Directory {
        pub(in super::super) fn lock(root: &Path, create: bool) -> Result<Self, String> {
            let root = File::from(
                fs::open(
                    root,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(io)?,
            );
            let state = directory(&root, ".gameskills", create)?;
            let directory = directory(&state, "queues", create)?;
            let lock = File::from(
                fs::openat(
                    &directory,
                    ".lock",
                    OFlags::RDWR
                        | OFlags::CREATE
                        | OFlags::NOFOLLOW
                        | OFlags::NONBLOCK
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(io)?,
            );
            ordinary(&lock)?;
            lock.lock().map_err(io)?;
            Ok(Self {
                directory,
                _lock: lock,
            })
        }
        pub(in super::super) fn exists(&self, name: &str) -> Result<bool, String> {
            match fs::statat(&self.directory, name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(_) => Ok(true),
                Err(e) if e == rustix::io::Errno::NOENT => Ok(false),
                Err(e) => Err(io(e)),
            }
        }
        pub(in super::super) fn read(&self, name: &str) -> Result<Value, String> {
            let file = File::from(
                fs::openat(
                    &self.directory,
                    name,
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(io)?,
            );
            ordinary(&file)?;
            read_json(file)
        }
        pub(in super::super) fn write(&self, name: &str, value: &Value) -> Result<(), String> {
            if self.exists(name)? {
                let file = File::from(
                    fs::openat(
                        &self.directory,
                        name,
                        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(io)?,
                );
                ordinary(&file)?;
            }
            let temp = format!(
                ".queue-{}-{}.tmp",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            );
            let mut file = File::from(
                fs::openat(
                    &self.directory,
                    &temp,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(io)?,
            );
            let result = (|| {
                let mut value = value.clone();
                value.sort_all_objects();
                let bytes = serde_json::to_vec_pretty(&value).map_err(io)?;
                if bytes.len() as u64 + 1 > LIMIT {
                    return Err("queue JSON exceeds 16 MiB limit".into());
                }
                file.write_all(&bytes).map_err(io)?;
                file.write_all(b"\n").map_err(io)?;
                file.sync_all().map_err(io)?;
                fs::renameat(&self.directory, &temp, &self.directory, name).map_err(io)?;
                self.directory.sync_all().map_err(io)
            })();
            if result.is_err() {
                let _ = fs::unlinkat(&self.directory, &temp, AtFlags::empty());
            }
            result
        }
        pub(in super::super) fn reject_active_legacy(&self) -> Result<(), String> {
            for entry in fs::Dir::read_from(&self.directory).map_err(io)? {
                let entry = entry.map_err(io)?;
                let name = entry.file_name().to_str().map_err(io)?;
                if !name.ends_with(".json") {
                    continue;
                }
                let value = self.read(name)?;
                match value.get("schema_version").and_then(Value::as_u64){
                    Some(1)=>{
                        let orders=value.get("orders").and_then(Value::as_object).filter(|o|!o.is_empty()).ok_or_else(||format!("cannot determine historical queue state in {name}"))?;
                        if orders.values().any(|o|o.get("state").and_then(Value::as_str)!=Some("integrated")){return Err(format!("historical Python queue {name} has unfinished orders; finish it with the original runtime before starting or changing Rust queues"));}
                    },
                    Some(super::super::QUEUE_SCHEMA) if value.get("runtime").and_then(Value::as_str)==Some(super::super::RUNTIME)=>{},
                    _=>return Err(format!("unsupported queue schema/runtime in {name}; preserve the original and review compatibility before mutation")),
                }
            }
            Ok(())
        }
    }
}
#[cfg(unix)]
pub(super) use posix::Directory;
#[cfg(not(unix))]
pub(super) struct Directory;
#[cfg(not(unix))]
impl Directory {
    pub(super) fn lock(_root: &Path, _create: bool) -> Result<Self, String> {
        Err("queue operations require POSIX advisory locking; Windows is unsupported".into())
    }
    pub(super) fn read(&self, _name: &str) -> Result<Value, String> {
        Err("Windows queue operations are unsupported".into())
    }
    pub(super) fn exists(&self, _name: &str) -> Result<bool, String> {
        Err("Windows queue operations are unsupported".into())
    }
    pub(super) fn write(&self, _name: &str, _value: &Value) -> Result<(), String> {
        Err("Windows queue operations are unsupported".into())
    }
    pub(super) fn reject_active_legacy(&self) -> Result<(), String> {
        Err("Windows queue operations are unsupported".into())
    }
}

//! Shared structural validation input handling. These are read-only helpers.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Read an ordinary UTF-8 source file, refusing symlink leaves and special files.
pub fn read_text(path: &Path) -> Result<String, String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("{}: expected an ordinary file", path.display()));
    }
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| UniqueValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = access.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, UniqueValue(value))) =
                    access.next_entry::<String, UniqueValue>()?
                {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key {key:?}")));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

/// Parse a JSON value without silently accepting duplicate keys at any depth.
pub fn parse_json(source: &str) -> Result<Value, String> {
    serde_json::from_str::<UniqueValue>(source)
        .map(|value| value.0)
        .map_err(|error| error.to_string())
}

/// Read a JSON object, rejecting duplicate keys and non-object roots.
pub fn read_json(path: &Path) -> Result<Value, String> {
    let value =
        parse_json(&read_text(path)?).map_err(|error| format!("{}: {error}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{}: expected a JSON object", path.display()));
    }
    Ok(value)
}

/// Enumerate repository source files deterministically without following directories through symlinks.
pub fn source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            let name = entry.file_name();
            if kind.is_dir() {
                if ![".git", ".context", ".gameskills", "target", "__pycache__"]
                    .iter()
                    .any(|ignored| name == *ignored)
                {
                    pending.push(entry.path());
                }
            } else if kind.is_file() || kind.is_symlink() && !entry.path().is_dir() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Check a Cargo/package entry using portable relative path rules.
pub fn portable_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

/// Resolve an existing local Markdown target within its owning boundary.
/// HTTP(S) and mail links, query-only references and anchors have no local target.
pub fn local_target(boundary: &Path, source: &Path, raw: &str) -> Result<Option<PathBuf>, String> {
    if raw.starts_with("http:") || raw.starts_with("https:") || raw.starts_with("mailto:") {
        return Ok(None);
    }
    if raw.starts_with('/') || raw.contains(['\\', '\0']) {
        return Err(format!("nonportable local path {raw:?}"));
    }
    let path = raw.split(['#', '?']).next().unwrap_or_default();
    if path.is_empty() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    let mut input = path.bytes();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let high = input
                .next()
                .and_then(|value| char::from(value).to_digit(16));
            let low = input
                .next()
                .and_then(|value| char::from(value).to_digit(16));
            let (Some(high), Some(low)) = (high, low) else {
                return Err(format!("invalid percent encoding in {raw:?}"));
            };
            bytes.push(u8::try_from(high * 16 + low).map_err(|error| error.to_string())?);
        } else {
            bytes.push(byte);
        }
    }
    let decoded = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    if decoded.starts_with('/') || decoded.contains(['\\', ':', '\0']) {
        return Err(format!("unsupported nonportable local link {raw:?}"));
    }
    let root = boundary.canonicalize().map_err(|error| error.to_string())?;
    let parent = source.parent().ok_or("source has no parent directory")?;
    let target = parent
        .join(decoded)
        .canonicalize()
        .map_err(|error| format!("missing local link {raw:?}: {error}"))?;
    if !target.starts_with(root) {
        return Err(format!("link escapes owning boundary: {raw}"));
    }
    Ok(Some(target))
}

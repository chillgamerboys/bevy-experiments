//! Validated launch profiles and exclusive, process-lifetime credential directories.

use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

/// Public launch options. Admission secrets are never accepted on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchOptions {
    /// Control the full company locally without opening a network session.
    pub local: bool,
    /// Deterministic initial encounter seed.
    pub seed: u64,
    /// Validated application profile, isolating local test clients.
    pub profile: String,
    /// Optional application data root; each profile gets its own child directory.
    pub data_dir: Option<PathBuf>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            local: false,
            seed: 42,
            profile: "default".to_owned(),
            data_dir: None,
        }
    }
}

impl LaunchOptions {
    /// Parses arguments after the executable name, rejecting unknown or repeated flags.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, LaunchError> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        let mut seen = BTreeSet::new();
        while let Some(argument) = args.next() {
            if !seen.insert(argument.clone()) {
                return Err(LaunchError::InvalidArguments);
            }
            match argument.as_str() {
                "--local" => options.local = true,
                "--seed" => {
                    options.seed = args
                        .next()
                        .ok_or(LaunchError::InvalidArguments)?
                        .parse()
                        .map_err(|_error| LaunchError::InvalidArguments)?;
                }
                "--profile" => {
                    options.profile = args.next().ok_or(LaunchError::InvalidArguments)?;
                    validate_profile(&options.profile)?;
                }
                "--data-dir" => {
                    let path = args.next().ok_or(LaunchError::InvalidArguments)?;
                    if path.is_empty() || path.starts_with("--") {
                        return Err(LaunchError::InvalidArguments);
                    }
                    options.data_dir = Some(PathBuf::from(path));
                }
                "--help" | "-h" => return Err(LaunchError::HelpRequested),
                _ => return Err(LaunchError::InvalidArguments),
            }
        }
        Ok(options)
    }
}

fn validate_profile(profile: &str) -> Result<(), LaunchError> {
    if profile.is_empty()
        || profile.len() > 32
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(LaunchError::InvalidProfile);
    }
    Ok(())
}

/// Holds an exclusive operating-system lock until the running game exits.
///
/// A crashed process releases the lock automatically; the harmless lock file remains.
/// Keep this value alive for the entire `App::run` call.
#[derive(Debug)]
pub struct ProfileGuard {
    _lock: File,
    directory: PathBuf,
    profile: String,
    local: bool,
}

impl ProfileGuard {
    /// Creates a private profile directory and fails if another process owns it.
    pub fn begin(options: &LaunchOptions) -> Result<Self, LaunchError> {
        validate_profile(&options.profile)?;
        let root = options
            .data_dir
            .clone()
            .map_or_else(default_data_root, Ok)?;
        let directory = root.join("profiles").join(&options.profile);
        fs::create_dir_all(&directory).map_err(|_error| LaunchError::StorageUnavailable)?;
        if fs::symlink_metadata(&directory)
            .map_err(|_error| LaunchError::StorageUnavailable)?
            .file_type()
            .is_symlink()
        {
            return Err(LaunchError::StorageUnavailable);
        }
        let lock_path = directory.join("profile.lock");
        if fs::symlink_metadata(&lock_path).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(LaunchError::StorageUnavailable);
        }
        let mut open = OpenOptions::new();
        open.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            open.mode(0o600);
        }
        let lock = open
            .open(lock_path)
            .map_err(|_error| LaunchError::StorageUnavailable)?;
        lock.try_lock()
            .map_err(|_error| LaunchError::ProfileInUse)?;
        Ok(Self {
            _lock: lock,
            directory,
            profile: options.profile.clone(),
            local: options.local,
        })
    }

    /// Profile-local rotating reconnect credential file, never shared across clients.
    #[must_use]
    pub fn credential_path(&self) -> PathBuf {
        self.directory.join("reconnect.json")
    }

    /// Directory for this instance's non-secret diagnostic artifacts.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Human-visible title identifying the exact local instance.
    #[must_use]
    pub fn title(&self) -> String {
        let mode = if self.local {
            "Local · full company"
        } else {
            "Co-op"
        };
        format!("Labyrinth · {} · {mode}", self.profile)
    }
}

fn default_data_root() -> Result<PathBuf, LaunchError> {
    directories::ProjectDirs::from("dev", "Gamekit", "Labyrinth")
        .map(|project| project.data_local_dir().to_owned())
        .ok_or(LaunchError::StorageUnavailable)
}

/// Safe launch diagnostics omit arbitrary argument values and filesystem details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    /// Show the documented command-line interface.
    HelpRequested,
    /// Unknown, repeated, or malformed option; no argument contents are echoed.
    InvalidArguments,
    /// Profile names must be short ASCII letters, numbers, hyphens or underscores.
    InvalidProfile,
    /// Profile storage could not be created safely.
    StorageUnavailable,
    /// Another instance owns this profile, or the platform rejected exclusive locking.
    ProfileInUse,
}

impl fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::HelpRequested => "Labyrinth [--local] [--seed INTEGER] [--profile NAME] [--data-dir PATH]\nProfiles isolate reconnect state. Use host and guest-a through guest-e for local multiplayer tests. Never pass admission secrets on the command line.",
            Self::InvalidArguments => "invalid launch arguments; use --help (admission secrets are not command-line options)",
            Self::InvalidProfile => "profile must contain 1-32 ASCII letters, numbers, hyphens or underscores",
            Self::StorageUnavailable => "profile storage is unavailable; try an explicit --data-dir",
            Self::ProfileInUse => "profile is already in use or could not be locked; select another --profile",
        })
    }
}

impl std::error::Error for LaunchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_validated_without_accepting_or_echoing_secrets() {
        let parsed = LaunchOptions::parse(
            ["--local", "--seed", "91", "--profile", "guest-a"].map(str::to_owned),
        )
        .expect("valid options");
        assert!(parsed.local);
        assert_eq!(parsed.seed, 91);
        assert_eq!(parsed.profile, "guest-a");
        for args in [
            vec!["--profile", "../host"],
            vec!["--profile", ""],
            vec!["--local", "--local"],
            vec!["--seed", "not-a-number"],
            vec!["--password", "do-not-echo-this"],
            vec!["--code", "BGN1-do-not-echo-this"],
        ] {
            let error = LaunchOptions::parse(args.into_iter().map(str::to_owned))
                .expect_err("invalid options rejected");
            assert!(!format!("{error:?} {error}").contains("do-not-echo-this"));
        }
    }

    #[test]
    fn profiles_isolate_credentials_and_recover_the_lock_after_drop() {
        let scratch = tempfile::tempdir().expect("profile test directory");
        let options = LaunchOptions {
            profile: "guest-a".to_owned(),
            data_dir: Some(scratch.path().to_owned()),
            ..LaunchOptions::default()
        };
        let first = ProfileGuard::begin(&options).expect("first instance");
        assert!(matches!(
            ProfileGuard::begin(&options),
            Err(LaunchError::ProfileInUse)
        ));
        let second = ProfileGuard::begin(&LaunchOptions {
            profile: "guest-b".to_owned(),
            ..options.clone()
        })
        .expect("independent second profile");
        assert_ne!(first.credential_path(), second.credential_path());
        assert!(first.title().contains("guest-a"));
        drop(first);
        assert!(ProfileGuard::begin(&options).is_ok());
    }
}

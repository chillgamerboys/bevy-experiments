//! Small OS boundaries for read-only foundation commands.
//!
//! Process supervision, lock ownership and atomic mutation are later migration work.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Read an ordinary UTF-8 file, refusing a symlink or special file at this path.
///
/// This read-only helper is not a race-safe filesystem capability for future writes.
pub fn read_ordinary_file(path: &Path) -> io::Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected an ordinary file",
        ));
    }
    fs::read_to_string(path)
}

/// An argument vector and working directory for a short-lived process.
#[derive(Debug)]
pub struct ProcessSpec {
    /// Executable path or name, resolved by the OS.
    pub program: OsString,
    /// Literal arguments; no shell parses these values.
    pub args: Vec<OsString>,
    /// Explicit process working directory.
    pub cwd: PathBuf,
}

impl ProcessSpec {
    /// Capture a short-lived process and wait for it.
    ///
    /// There is no timeout, descendant cleanup or resource-lock guarantee here.
    /// Never use this helper as the later command runner or native-client supervisor.
    pub fn output(&self) -> io::Result<Output> {
        Command::new(&self.program)
            .args(&self.args)
            .current_dir(&self.cwd)
            .output()
    }
}

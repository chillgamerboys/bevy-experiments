//! Command boundary and versioned diagnostics for the foundation candidate.

use clap::{Parser, Subcommand};
use serde_json::json;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "gameskills",
    version,
    about = "GameSkills Rust foundation; installation and execution are not ported yet"
)]
struct Arguments {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Inspect configuration. Only the explicit validate subcommand is implemented.
    Config {
        #[command(subcommand)]
        command: Option<ConfigOperation>,
    },
    /// Installation catalog (not yet ported).
    Catalog,
    /// Installation readiness (not yet ported).
    Status,
    /// Install or recover a bundle (not yet ported).
    Setup {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Prepare an immutable bundle (not yet ported).
    Bundle {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Activate an agent client (not yet ported).
    Native {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Validate a work plan (not yet ported).
    Plan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Coordinate durable work (not yet ported).
    Queue {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Execute configured commands (not yet ported).
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Inspect execution records (not yet ported).
    Evidence {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Import an older installation (not yet ported).
    Legacy {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
}

#[derive(Subcommand)]
enum ConfigOperation {
    /// Validate a TOML file without setup, writes, or execution. Does not establish readiness.
    Validate {
        /// Configuration file, relative to --root unless absolute.
        #[arg(long, default_value = "gameskills.toml")]
        file: PathBuf,
    },
}

/// One command result; output is JSON except for help/version.
pub struct Response {
    /// Process exit status, zero only for the requested supported operation.
    pub exit_code: u8,
    /// Human-readable help/version, or one JSON document.
    pub output: String,
}

fn failure(code: &str, message: impl ToString) -> Response {
    Response { exit_code: 2, output: json!({"schema_version": 1, "ok": false, "error": {"code": code, "message": message.to_string()}}).to_string() }
}

/// Parse an argument vector and perform only implemented read-only operations.
pub fn execute(args: impl IntoIterator<Item = OsString>) -> Response {
    let parsed = match Arguments::try_parse_from(args) {
        Ok(parsed) => parsed,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            return Response {
                exit_code: 0,
                output: error.to_string().trim_end().into(),
            };
        }
        Err(error) => return failure("invalid_arguments", error),
    };
    match parsed.command {
        Operation::Config { command: Some(ConfigOperation::Validate { file }) } => {
            let path = parsed.root.join(file);
            let source = match crate::platform::read_ordinary_file(&path) {
                Ok(source) => source,
                Err(error) => return failure("configuration_io", error),
            };
            match crate::config::parse(&source) {
                Ok(configuration) => Response { exit_code: 0, output: json!({"schema_version": 1, "ok": true, "scope": "configuration_structure", "configuration": configuration}).to_string() },
                Err(error) => failure("invalid_configuration", error),
            }
        }
        _ => failure("not_implemented", "this operation is not ported; use the existing pinned candidate until its Rust replacement is accepted"),
    }
}

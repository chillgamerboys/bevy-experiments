//! Repository maintenance executable entrypoint.

use clap::{Parser, Subcommand};
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "gamekit-repo",
    version,
    about = "Internal Rust migration contract checker; repository/CI commands are not ported yet"
)]
struct Arguments {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Inspect the R0 migration ledger.
    Contracts {
        #[command(subcommand)]
        command: ContractOperation,
    },
    /// Layout and links (not yet ported).
    Check,
    /// Catalog validation (not yet ported).
    Skills {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
    /// Distribution verification (not yet ported).
    Distribution {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
    /// CI selection and gating (not yet ported).
    Ci {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
}

#[derive(Subcommand)]
enum ContractOperation {
    /// Check complete file/test accounting, without asserting that ports have passed.
    Check {
        /// Check frozen source digests and symbols against the original Git objects.
        #[arg(long)]
        verify_reference: bool,
    },
}

fn failure(code: &str, error: impl ToString) -> (u8, String) {
    (2, json!({"schema_version": 1, "ok": false, "error": {"code": code, "message": error.to_string()}}).to_string())
}

fn execute() -> (u8, String) {
    let args = match Arguments::try_parse() {
        Ok(args) => args,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            return (0, error.to_string().trim_end().into())
        }
        Err(error) => return failure("invalid_arguments", error),
    };
    match args.command {
        Operation::Contracts {
            command: ContractOperation::Check { verify_reference },
        } => {
            let path = args.root.join("tools/migration-contracts.json");
            let source = match std::fs::symlink_metadata(&path).and_then(|metadata| {
                if metadata.file_type().is_file() {
                    std::fs::read_to_string(&path)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "expected an ordinary inventory file",
                    ))
                }
            }) {
                Ok(source) => source,
                Err(error) => return failure("inventory_io", error),
            };
            if let Err(error) = gamekit_repo_tools::contracts::validate(&source) {
                return failure("invalid_inventory", error);
            }
            if verify_reference {
                if let Err(error) = gamekit_repo_tools::contracts::verify_reference(&args.root) {
                    return failure("reference_unavailable_or_changed", error);
                }
            }
            (0, json!({"schema_version": 1, "ok": true, "scope": "migration_accounting", "source_files": 22, "test_methods": 142, "reference_commit": gamekit_repo_tools::contracts::REFERENCE, "reference_verified": verify_reference, "ports_verified": false}).to_string())
        }
        _ => failure(
            "not_implemented",
            "this repository command has not been ported; retain the current configured check",
        ),
    }
}

fn main() -> ExitCode {
    let (status, output) = execute();
    if writeln!(io::stdout().lock(), "{output}").is_err() {
        return ExitCode::from(2);
    }
    ExitCode::from(status)
}

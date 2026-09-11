//! Repository maintenance executable entrypoint.

use clap::{Parser, Subcommand};
use gamekit_repo_tools::{catalog, distribution, legacy, repository};
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "gamekit-repo",
    version,
    about = "Internal repository, skill catalog and distribution checks"
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
    /// Check workspace layout, local links and capability dependency ownership.
    Check,
    /// Validate candidate and legacy skill sources without executing an agent.
    Skills {
        #[command(subcommand)]
        command: SkillOperation,
    },
    /// Verify staged external consumers of the library packages.
    Distribution {
        #[command(subcommand)]
        command: DistributionOperation,
    },
    /// CI selection and gating (not yet ported).
    Ci {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
}

#[derive(Subcommand)]
enum SkillOperation {
    /// Check all six native packages, metadata, links and scenario rubrics.
    Validate {
        /// Select JSON explicitly; JSON is also the default output format.
        #[arg(long)]
        json: bool,
    },
    /// Check the seven canonical legacy skill sources and trigger fixtures.
    Legacy,
}

#[derive(Subcommand)]
enum DistributionOperation {
    /// Stage Cargo-selected library sources and build the selected consumer cases.
    Check {
        #[arg(long, value_enum, default_value = "all")]
        case: distribution::Case,
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
        Operation::Check => {
            let failures = repository::check(&args.root);
            (u8::from(!failures.is_empty()), json!({"schema_version":1,"ok":failures.is_empty(),"scope":"repository_structure","failures":failures}).to_string())
        }
        Operation::Skills { command: SkillOperation::Validate { .. } } => {
            let failures = catalog::validate(&args.root);
            let skills: usize = catalog::EXPECTED_SKILLS.iter().map(|(_, skills)| skills.len()).sum();
            let core = catalog::EXPECTED_SKILLS.iter().find(|(name, _)| *name == "gameskills").map_or(0, |(_, skills)| skills.len());
            (u8::from(!failures.is_empty()), json!({"schema_version":1,"ok":failures.is_empty(),"structural_only":true,"packages":catalog::EXPECTED_SKILLS.len(),"skills":skills,"core_skills":core,"optional_skills":skills-core,"failures":failures,"notice":catalog::NOTICE}).to_string())
        }
        Operation::Skills { command: SkillOperation::Legacy } => {
            let failures = legacy::validate(&args.root);
            (u8::from(!failures.is_empty()), json!({"schema_version":1,"ok":failures.is_empty(),"structural_only":true,"skills":legacy::SKILLS.len(),"clients":legacy::CLIENTS,"failures":failures,"notice":legacy::NOTICE}).to_string())
        }
        Operation::Distribution { command: DistributionOperation::Check { case } } => {
            match distribution::check(&args.root, case) {
                Ok(report) => (0, json!({"schema_version":1,"ok":true,"scope":"external_library_consumers","report":report}).to_string()),
                Err(error) => (1, json!({"schema_version":1,"ok":false,"error":{"code":"distribution_failed","message":error}}).to_string()),
            }
        }
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
